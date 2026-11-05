pub mod commands;
mod stop_words;
pub mod texts;
pub mod index;
mod writer;
mod search;

use pali_analysis::analyzers::{AnalyzerConfig, stop_word_filter};
use crate::commands::Stemmer;
use pali_analysis::dict_stemmer::DictionaryMode;
use pali_analysis::dpd::Dictionary;
use crate::index::{Location, PaliIndex};
use crate::stop_words::most_frequent_words;
use pali_analysis::table::StemTable;
use crate::texts::PaliFiles;
use pali_analysis::tokenizer::{PaliTokenizer, nfc};
use pali_analysis::vocabulary::Vocabulary;
use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, ValueEnum};
use commands::{Arguments, Command};
use itertools::join;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::{canonicalize, create_dir_all, read, read_dir, read_to_string, remove_dir_all, rename, write};
use std::io::{BufRead, BufWriter, ErrorKind, Write, stdin, stdout};
use std::path::{Path, PathBuf};
use tabled::settings::Style;
use tabled::Table;
use tantivy::tokenizer::{LowerCaser, TextAnalyzer, Token, TokenStream};
use crate::search::{PaliSearcher, SearchResult};
use crate::writer::PaliWriter;

const STEMMER_FILE: &str = "stemmer.txt";
/// Tantivy's own record, in an index built before `stemmer.txt` existed.
const TANTIVY_FILE: &str = "meta.json";
const DEFAULT_STEMMER: Stemmer = Stemmer::Snowball;

fn main() -> Result<()> {
    match run(Arguments::parse()) {
        // A reader that stops early, as `pali stem-words | head` does, is not an error.
        Err(error) if is_broken_pipe(&error) => Ok(()),
        done => done,
    }
}

fn is_broken_pipe(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == ErrorKind::BrokenPipe))
}

fn run(args: Arguments) -> Result<()> {
    match args.command {
        Command::StemTable => {
            create_stem_table(&args.texts, &args.dpd_db, &args.stem_file)?;
        }
        Command::StemWords => {
            stem_words(args.stemmer.unwrap_or(DEFAULT_STEMMER), &args.stem_file)?;
        }
        Command::Analyze { text } => {
            analyze_text(args.stemmer.unwrap_or(DEFAULT_STEMMER), text.as_str(), &args.stem_file)?;
        }
        Command::PaliChars => {
            show_pali_characters()?;
        }
        Command::OtherChars => {
            show_other_characters(args.texts)?;
        }
        Command::StopWords { number } => {
            show_stop_words(args.texts, number)?;
        }
        Command::Index => {
            create_index(args.texts, &args.index_path, &args.stem_file, args.stemmer.unwrap_or(DEFAULT_STEMMER))?;
        }
        Command::Find { query } => {
            search_index(&args.index_path, &args.stem_file, args.stemmer, query)?;
        }
    }
    Ok(())
}

fn create_stem_table(
    texts_path: &Path,
    dictionary_path: &Path,
    stem_file_path: &Path,
) -> Result<()> {
    let files = PaliFiles::new(PathBuf::from(texts_path));
    let analyzer = TextAnalyzer::builder(PaliTokenizer::default())
        .filter(LowerCaser)
        .filter(stop_word_filter())
        .build();
    let vocabulary = Vocabulary::new(files.all_segments()?.into_iter().map(|segment| segment.text), analyzer);
    if vocabulary.is_empty() {
        bail!("found no Pali texts at {}", texts_path.display());
    }
    let dictionary = Dictionary::open(dictionary_path)?;
    dictionary.check_preferred_roots()?;
    let term_stems = StemTable::new(vocabulary, &dictionary)?;
    term_stems.save(stem_file_path)?;
    Ok(())
}

fn read_stem_table(stem_file: &Path) -> Result<Vec<u8>> {
    read(stem_file).with_context(|| format!("reading stem file {}", stem_file.display()))
}

/// The analyzer for a stemmer, with a table's bytes for those that need one.
fn analyzer_config(stemmer: Stemmer, stem_table: Option<Vec<u8>>) -> AnalyzerConfig {
    let mode = match stemmer {
        Stemmer::Snowball => return AnalyzerConfig::Algorithmic,
        Stemmer::Dictionary => DictionaryMode::Plain,
        Stemmer::Hybrid => DictionaryMode::Hybrid,
        Stemmer::Family => DictionaryMode::Family,
    };
    AnalyzerConfig::Dictionary {
        stem_table: stem_table.unwrap_or_default(),
        mode,
    }
}

fn stem_table_for(stemmer: Stemmer, stem_file: &Path) -> Result<Option<Vec<u8>>> {
    if stemmer == Stemmer::Snowball {
        Ok(None)
    } else {
        read_stem_table(stem_file).map(Some)
    }
}

fn stem_words(stemmer: Stemmer, stem_file_path: &Path) -> Result<()> {
    let table = stem_table_for(stemmer, stem_file_path)?;
    let mut analyzer = TextAnalyzer::try_from(analyzer_config(stemmer, table))?;
    let mut out = BufWriter::new(stdout().lock());
    for line in stdin().lock().lines() {
        let line = line?;
        let mut stems = Vec::new();
        analyzer
            .token_stream(&nfc(&line))
            .process(&mut |token: &Token| stems.push(token.text.clone()));
        writeln!(out, "{line}\t{}", stems.join(" "))?;
    }
    out.flush()?;
    Ok(())
}

fn analyze_text(stemmer: Stemmer, text: &str, stem_file_path: &Path) -> Result<()> {
    let table = stem_table_for(stemmer, stem_file_path)?;
    let mut analyzer = TextAnalyzer::try_from(analyzer_config(stemmer, table))?;
    let text = nfc(text);
    let mut terms = String::new();
    analyzer.token_stream(&text).process(&mut |token: &Token| {
        terms.push_str(&token.text);
        terms.push(' ');
    });
    write!(stdout().lock(), "{terms}")?;
    Ok(())
}

fn show_pali_characters() -> Result<()> {
    let tokenizer = PaliTokenizer::default();
    let mut alphabet: Vec<char> = tokenizer.alphabet.into_iter().collect();
    alphabet.sort_unstable();
    let mut out = stdout().lock();
    for char in alphabet {
        write!(out, "{char} ")?;
    }
    Ok(())
}

/// Like `index` and `stem-table`, these refuse texts they cannot read rather than
/// report on the rest as if it were everything.
fn show_other_characters(texts_path: PathBuf) -> Result<()> {
    let files = PaliFiles::new(texts_path);
    let mut chars = BTreeSet::<char>::new();
    for segment in files.all_segments()? {
        for char in segment.text.chars() {
            chars.insert(char);
        }
    }
    let tokenizer = PaliTokenizer::default();
    let mut out = stdout().lock();
    for char in chars {
        if !tokenizer.alphabet.contains(&char) {
            write!(out, "{char} ")?;
        }
    }
    Ok(())
}

fn show_stop_words(texts_path: PathBuf, number: usize) -> Result<()> {
    let files = PaliFiles::new(texts_path);
    let mut out = stdout().lock();
    for word in most_frequent_words(files.all_segments()?.into_iter(), number) {
        writeln!(out, "{word}")?;
    }
    Ok(())
}

/// A directory next to `path`, named after it.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().map_or_else(OsString::new, OsStr::to_os_string);
    name.push(suffix);
    path.with_file_name(name)
}

/// Refuse to remove a directory that holds anything but an index's own files, so that
/// a stray meta.json cannot carry a thesis away with it.
fn check_replaceable(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if read_dir(path)?.next().is_none() {
        return Ok(());
    }
    // Every file is checked by name first, so that nothing else in the directory is read.
    for entry in read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name();
        if !entry.file_type()?.is_file() || !name.to_str().is_some_and(is_index_file) {
            bail!("{} holds {} as well as an index, so it will not be replaced", path.display(), name.to_string_lossy());
        }
    }
    if !path.join(TANTIVY_FILE).is_file() || !PaliIndex::is_ours(path) {
        bail!("{} holds no index of this program's, so it will not be replaced", path.display());
    }
    Ok(())
}

const TANTIVY_EXTENSIONS: [&str; 7] = ["idx", "pos", "term", "store", "fast", "fieldnorm", "del"];

/// A file Tantivy or this program writes into an index: meta.json, its locks, and
/// segment files named by a 32-digit hex id, such as 5f48….idx or 5f48….12.del.
fn is_index_file(name: &str) -> bool {
    if [STEMMER_FILE, TANTIVY_FILE, ".managed.json", ".tantivy-meta.lock", ".tantivy-writer.lock"].contains(&name) {
        return true;
    }
    let mut parts = name.split('.');
    let id = parts.next().unwrap_or_default();
    let extension = name.rsplit('.').next().unwrap_or_default();
    id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit()) && TANTIVY_EXTENSIONS.contains(&extension)
}

/// Build beside the old index and swap, so that a failure leaves the old index in place.
fn create_index(texts_path: PathBuf, index_path: &Path, stem_file_path: &Path, stemmer: Stemmer) -> Result<()> {
    if !texts_path.is_dir() {
        bail!("no texts at {}", texts_path.display());
    }
    if index_path.file_name().is_none() {
        bail!("{} cannot be replaced: name a directory for the index", index_path.display());
    }
    // A link stays a link: the index goes where it points.
    let resolved = if index_path.exists() { canonicalize(index_path)? } else { index_path.to_path_buf() };
    let index_path = resolved.as_path();
    let building = sibling(index_path, ".building");
    for path in [index_path, &building, &sibling(index_path, ".previous")] {
        check_replaceable(path)?;
    }
    let table = stem_table_for(stemmer, stem_file_path)?;
    let fingerprint = pali_analysis::fingerprint(table.as_deref());
    let analyzer = TextAnalyzer::try_from(analyzer_config(stemmer, table))?;

    if building.exists() {
        remove_dir_all(&building)?;
    }
    create_dir_all(&building)?;
    let done = build_index(texts_path, &building, analyzer, stemmer, fingerprint)
        .and_then(|()| swap_in(&building, index_path));
    if done.is_err() && building.exists() {
        // The first failure is the one to report, whether or not this works.
        let _ = remove_dir_all(&building);
    }
    done
}

fn build_index(
    texts_path: PathBuf,
    building: &Path,
    analyzer: TextAnalyzer,
    stemmer: Stemmer,
    fingerprint: u64,
) -> Result<()> {
    let files = PaliFiles::new(texts_path);
    files.check_readable()?;
    let index = PaliIndex::create(Location::InDir { index_path: building.to_path_buf() }, analyzer)?;
    let mut writer = PaliWriter::new(&index)?;
    let texts = writer.add_texts(files.texts())?;
    writer.finish()?;
    if texts == 0 {
        bail!("found no Pali texts to index");
    }
    write(building.join(STEMMER_FILE), format!("{}\n{fingerprint}\n", stemmer_name(stemmer)))?;
    Ok(())
}

fn swap_in(building: &Path, index_path: &Path) -> Result<()> {
    let previous = sibling(index_path, ".previous");
    if previous.exists() {
        remove_dir_all(&previous)?;
    }
    let replacing = index_path.exists();
    if replacing {
        rename(index_path, &previous)?;
    }
    if let Err(error) = rename(building, index_path) {
        if replacing {
            rename(&previous, index_path)?;
        }
        return Err(error.into());
    }
    if replacing {
        remove_dir_all(&previous)?;
    }
    Ok(())
}

fn stemmer_name(stemmer: Stemmer) -> String {
    stemmer.to_possible_value().map_or_else(String::new, |value| value.get_name().to_string())
}

/// The stemmer `index` recorded, read from its record.
fn recorded_stemmer(record: &str) -> Result<Stemmer> {
    let name = record.lines().next().unwrap_or_default();
    Stemmer::from_str(name, true).map_err(|error| anyhow!("unknown stemmer {name} recorded in the index: {error}"))
}

/// Refuse a search that would not match the index. Queries must be stemmed exactly
/// as the index was, or they silently find too little.
fn check_record(record: &str, stemmer: Stemmer, requested: Option<Stemmer>, table: Option<&[u8]>) -> Result<()> {
    if requested.is_some_and(|requested| requested != stemmer) {
        bail!(
            "this index was built with --stemmer {}, so search with that stemmer or run index again",
            stemmer_name(stemmer)
        );
    }
    if record.lines().nth(1) != Some(pali_analysis::fingerprint(table).to_string().as_str()) {
        bail!("the rules or stem table have changed since this index was built; run index again");
    }
    Ok(())
}

fn find(index_path: &Path, stem_file_path: &Path, requested: Option<Stemmer>, query: &str) -> Result<Vec<SearchResult>> {
    let record = read_to_string(index_path.join(STEMMER_FILE))
        .with_context(|| format!("{} has no record of its stemmer; run index again", index_path.display()))?;
    let stemmer = recorded_stemmer(&record)?;
    let table = stem_table_for(stemmer, stem_file_path)?;
    check_record(&record, stemmer, requested, table.as_deref())?;
    let analyzer = TextAnalyzer::try_from(analyzer_config(stemmer, table))?;
    let index = PaliIndex::open(index_path, analyzer)?;
    PaliSearcher::new(&index)?.search(query)
}

fn search_index(index_path: &Path, stem_file_path: &Path, requested: Option<Stemmer>, query: Vec<String>) -> Result<()> {
    let results = find(index_path, stem_file_path, requested, &join(query, " "))?;
    let mut table = Table::new(results);
    table.with(Style::modern());
    writeln!(stdout().lock(), "{table}")?;
    Ok(())
}

#[cfg(test)]
mod main_tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs::create_dir;
    use temp_dir::TempDir;

    #[test]
    fn test_sibling_keeps_the_whole_name() {
        assert_eq!(sibling(Path::new("data/index.v2"), ".building"), Path::new("data/index.v2.building"));
    }

    #[test]
    fn test_swap_replaces_the_old_index_and_leaves_nothing_behind() {
        let dir = TempDir::new().unwrap();
        let (old, new) = (dir.child("index"), dir.child("index.building"));
        create_dir(&old).unwrap();
        write(old.join("old"), "").unwrap();
        create_dir(&new).unwrap();
        write(new.join("new"), "").unwrap();
        swap_in(&new, &old).unwrap();
        assert!(old.join("new").exists() && !old.join("old").exists());
        assert!(!new.exists() && !dir.child("index.previous").exists());
    }

    #[test]
    fn test_failed_swap_restores_the_old_index() {
        let dir = TempDir::new().unwrap();
        let old = dir.child("index");
        create_dir(&old).unwrap();
        write(old.join("old"), "").unwrap();
        assert!(swap_in(&dir.child("missing"), &old).is_err());
        assert!(old.join("old").exists());
    }

    fn record(stemmer: Stemmer, table: Option<&[u8]>) -> String {
        format!("{}\n{}\n", stemmer_name(stemmer), pali_analysis::fingerprint(table))
    }

    #[test]
    fn test_search_takes_the_stemmer_from_the_index() {
        let found = recorded_stemmer(&record(Stemmer::Hybrid, None)).unwrap();
        assert_eq!(found, Stemmer::Hybrid);
    }

    #[test]
    fn test_search_refuses_a_different_stemmer() {
        let record = record(Stemmer::Hybrid, Some(b"t"));
        assert!(check_record(&record, Stemmer::Hybrid, Some(Stemmer::Family), Some(b"t")).is_err());
    }

    #[test]
    fn test_search_refuses_a_changed_table() {
        let record = record(Stemmer::Hybrid, Some(b"t"));
        assert!(check_record(&record, Stemmer::Hybrid, None, Some(b"other")).is_err());
    }

    #[test]
    fn test_index_goes_end_to_end_through_the_stem_table() {
        let dir = TempDir::new().unwrap();
        let texts = dir.child("texts");
        create_dir(&texts).unwrap();
        write(texts.join("mn1_root-pli-ms.json"), r#"{"mn1:1": "dhammo"}"#).unwrap();
        let stems = dir.child("stems.csv");
        write(&stems, "term,dpd_stem,snowball_stem,family,fallback\ndhammo,dhamma,dhamm,,dhamma\n").unwrap();
        let index = dir.child("index");
        create_index(texts, &index, &stems, Stemmer::Hybrid).unwrap();
        assert_eq!(find(&index, &stems, None, "dhammassa").unwrap().len(), 1);
        assert!(find(&index, &stems, None, "xyzzy").unwrap().is_empty());
        assert!(!sibling(&index, ".building").exists());
    }

    #[test]
    fn test_failed_build_leaves_nothing_and_keeps_the_old_index() {
        let dir = TempDir::new().unwrap();
        let texts = dir.child("texts");
        create_dir(&texts).unwrap();
        let index = dir.child("index");
        create_dir(&index).unwrap();
        write(index.join(STEMMER_FILE), "snowball\n1\n").unwrap();
        assert!(create_index(texts, &index, &dir.child("none"), Stemmer::Snowball).is_err());
        assert!(index.join(STEMMER_FILE).exists() && !sibling(&index, ".building").exists());
    }

    #[test]
    fn test_index_will_not_replace_another_directory() {
        let dir = TempDir::new().unwrap();
        let texts = dir.child("texts");
        create_dir(&texts).unwrap();
        let precious = dir.child("precious");
        create_dir(&precious).unwrap();
        write(precious.join("thesis.txt"), "").unwrap();
        assert!(create_index(texts, &precious, &dir.child("none"), Stemmer::Snowball).is_err());
        assert!(precious.join("thesis.txt").exists());
    }

    fn texts_dir(dir: &TempDir) -> PathBuf {
        let texts = dir.child("texts");
        create_dir(&texts).unwrap();
        write(texts.join("mn1_root-pli-ms.json"), r#"{"mn1:1": "dhammo"}"#).unwrap();
        texts
    }

    #[test]
    fn test_a_meta_file_does_not_make_a_directory_disposable() {
        let dir = TempDir::new().unwrap();
        let documents = dir.child("documents");
        create_dir(&documents).unwrap();
        write(documents.join(TANTIVY_FILE), "{}").unwrap();
        write(documents.join("thesis.txt"), "").unwrap();
        assert!(create_index(texts_dir(&dir), &documents, &dir.child("none"), Stemmer::Snowball).is_err());
        assert!(documents.join("thesis.txt").exists());
    }

    #[test]
    fn test_a_lone_settings_file_named_meta_is_kept() {
        let dir = TempDir::new().unwrap();
        let documents = dir.child("documents");
        create_dir(&documents).unwrap();
        write(documents.join(TANTIVY_FILE), r#"{"my": "settings"}"#).unwrap();
        assert!(create_index(texts_dir(&dir), &documents, &dir.child("none"), Stemmer::Snowball).is_err());
        assert_eq!(read_to_string(documents.join(TANTIVY_FILE)).unwrap(), r#"{"my": "settings"}"#);
    }

    /// An empty index of this program's, as Tantivy writes it, without a stemmer record.
    fn bare_index(path: &Path) {
        create_dir(path).unwrap();
        let analyzer = TextAnalyzer::builder(PaliTokenizer::default()).build();
        PaliIndex::create(Location::InDir { index_path: path.to_path_buf() }, analyzer).unwrap();
    }

    #[test]
    fn test_only_this_programs_tantivy_index_makes_a_directory_an_index() {
        let dir = TempDir::new().unwrap();
        for (n, (name, text)) in [
            (TANTIVY_FILE, r#"["segments", "index_settings", "notes"]"#),
            (TANTIVY_FILE, r#"{"segments": "x", "index_settings": {}}"#),
            (TANTIVY_FILE, r#"{"index_settings": {}, "segments": []}"#),
            (TANTIVY_FILE, r#"{"index_settings": {}, "segments": [], "notes": "unrelated"}"#),
            (STEMMER_FILE, "snowball\n1\n"),
            (".managed.json", "[]"),
        ]
        .into_iter()
        .enumerate()
        {
            let target = dir.child(format!("target{n}"));
            create_dir(&target).unwrap();
            write(target.join(name), text).unwrap();
            assert!(check_replaceable(&target).is_err(), "{name}: {text}");
            assert_eq!(read_to_string(target.join(name)).unwrap(), text);
        }
        // A real Tantivy index, but with another program's schema.
        let other = dir.child("other");
        create_dir(&other).unwrap();
        let mut schema = tantivy::schema::Schema::builder();
        schema.add_text_field("notes", tantivy::schema::TEXT);
        tantivy::Index::create_in_dir(&other, schema.build()).unwrap();
        assert!(check_replaceable(&other).is_err());
        let index = dir.child("index");
        bare_index(&index);
        let before: BTreeSet<_> = read_dir(&index).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert!(check_replaceable(&index).is_ok());
        let after: BTreeSet<_> = read_dir(&index).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(before, after, "checking an index must not write into it");
        let empty = dir.child("empty");
        create_dir(&empty).unwrap();
        assert!(check_replaceable(&empty).is_ok());
    }

    #[test]
    fn test_index_files_are_recognised_by_name() {
        assert!(is_index_file("5f48de0d039844ca9137c4386ab491a5.idx"));
        assert!(is_index_file("5f48de0d039844ca9137c4386ab491a5.12.del"));
        assert!(is_index_file(".managed.json"));
        assert!(!is_index_file("5f48de0d039844ca9137c4386ab491a5.txt"));
        assert!(!is_index_file("notes.idx"));
        assert!(!is_index_file("thesis.txt"));
    }

    #[test]
    fn test_a_rebuilt_index_can_be_rebuilt_again() {
        let dir = TempDir::new().unwrap();
        let index = dir.child("index");
        create_index(texts_dir(&dir), &index, &dir.child("none"), Stemmer::Snowball).unwrap();
        create_index(dir.child("texts"), &index, &dir.child("none"), Stemmer::Snowball).unwrap();
    }

    #[test]
    fn test_an_index_from_before_the_record_can_be_replaced() {
        let dir = TempDir::new().unwrap();
        let index = dir.child("index");
        bare_index(&index);
        create_index(texts_dir(&dir), &index, &dir.child("none"), Stemmer::Snowball).unwrap();
        assert!(index.join(STEMMER_FILE).exists());
    }

    #[test]
    fn test_index_will_not_clear_a_directory_beside_it() {
        let dir = TempDir::new().unwrap();
        let index = dir.child("index");
        for suffix in [".building", ".previous"] {
            let beside = sibling(&index, suffix);
            create_dir(&beside).unwrap();
            write(beside.join("thesis.txt"), "").unwrap();
        }
        let texts = texts_dir(&dir);
        assert!(create_index(texts, &index, &dir.child("none"), Stemmer::Snowball).is_err());
        assert!(sibling(&index, ".building").join("thesis.txt").exists());
    }

    #[test]
    fn test_index_needs_a_named_directory() {
        let dir = TempDir::new().unwrap();
        let error = create_index(texts_dir(&dir), Path::new("."), &dir.child("none"), Stemmer::Snowball).unwrap_err();
        assert!(error.to_string().contains("name a directory"), "{error}");
    }

    #[test]
    fn test_a_linked_index_stays_a_link() {
        let dir = TempDir::new().unwrap();
        let (real, link) = (dir.child("real"), dir.child("link"));
        create_dir(&real).unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        create_index(texts_dir(&dir), &link, &dir.child("none"), Stemmer::Snowball).unwrap();
        assert!(link.is_symlink() && real.join(STEMMER_FILE).exists());
    }

    #[test]
    fn test_a_dictionary_table_goes_end_to_end_into_a_family_index() {
        let dir = TempDir::new().unwrap();
        let db = dir.child("dpd.db");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch(
            "CREATE TABLE dpd_headwords (id INTEGER, lemma_1 TEXT, pos TEXT, root_key TEXT,
                family_root TEXT, root_base TEXT, construction TEXT);
            CREATE TABLE lookup (lookup_key TEXT, headwords TEXT, deconstructor TEXT);
            INSERT INTO dpd_headwords VALUES (1, 'gacchati 1', 'pr', '√gam 1', '√gam', '', ''),
                (2, 'gata 1', 'pp', '√gam 1', '√gam', '', ''),
                (3, 'vutta 2', 'pp', '√vac', '√vac', '', ''), (4, 'sati 1', 'fem', '√sar 2', '√sar', '', ''),
                (5, 'sutta 2', 'nt', '√siv 1', '√siv', '', ''), (6, 'attha 1', 'nt', '√atth', '√atth', '', ''),
                (7, 'kappa 1', 'masc', '√kapp 1', '√kapp', '', ''), (8, 'atthi', 'pr', '√as 1', '√as', '', ''),
                (9, 'suta 1', 'pp', '√su 1', '√su', '', ''), (10, 'sayaṃ', 'ind', '', '', '', ''),
                (11, 'kāya 1', 'masc', '√ci 1', '√ci', '', ''), (12, 'āyati', 'pr', '√yat 2', '√yat', '', ''),
                (13, 'asati', 'loc', '√as 1', '√as', '', ''), (14, 'asanta', 'prp', '√as 1', '√as', '', ''),
                (15, 'seyya 1.1', 'adj', '', '', '', ''), (16, 'dassati 1.1', 'fut', '√dā 1', '√dā', '', '');
            INSERT INTO lookup VALUES ('gacchati', '[1]', ''), ('gato', '[2]', '');",
        )
        .unwrap();
        drop(conn);
        let texts = dir.child("texts");
        create_dir(&texts).unwrap();
        write(texts.join("mn1_root-pli-ms.json"), r#"{"mn1:1": "gato"}"#).unwrap();
        let stems = dir.child("stems.csv");
        create_stem_table(&texts, &db, &stems).unwrap();
        let index = dir.child("index");
        create_index(texts, &index, &stems, Stemmer::Family).unwrap();
        assert_eq!(find(&index, &stems, None, "gacchati").unwrap().len(), 1);
    }

    #[test]
    fn test_missing_texts_are_not_readable() {
        let dir = TempDir::new().unwrap();
        assert!(PaliFiles::new(dir.child("none")).check_readable().is_err());
        assert!(PaliFiles::new(dir.path().to_path_buf()).check_readable().is_ok());
    }

}
