//! The `pali` command line, run as a user runs it: the built binary, real files, real
//! exit codes and messages.

use rusqlite::Connection;
use std::fs::{create_dir, read_dir, read_to_string, set_permissions, write};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use temp_dir::TempDir;

const STEM_HEADER: &str = "term,dpd_stem,snowball_stem\n";
const STEMS: &str = "term,dpd_stem,snowball_stem\n\
    dhammo,dhamma,dhamm\ngato,gata,gat\n";

/// `pali` run in `dir`, so that its default paths (data/dpd.db, index) fall inside it.
fn pali(dir: &Path, args: &[&str]) -> Output {
    pali_with_input(dir, args, "")
}

fn pali_with_input(dir: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_pali"))
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the pali binary runs");
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[track_caller]
fn succeeds(output: &Output) -> String {
    assert!(output.status.success(), "failed: {}", stderr(output));
    stdout(output)
}

/// The error message of a run that must fail.
#[track_caller]
fn fails(output: &Output) -> String {
    assert!(!output.status.success(), "succeeded: {}", stdout(output));
    stderr(output)
}

fn texts(dir: &TempDir, json: &str) -> PathBuf {
    let texts = dir.child("texts");
    create_dir(&texts).unwrap();
    write(texts.join("mn1_root-pli-ms.json"), json).unwrap();
    texts
}

fn stem_file(dir: &TempDir, contents: &str) -> PathBuf {
    let stems = dir.child("stems.csv");
    write(&stems, contents).unwrap();
    stems
}

fn s(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// Files left beside a stem table while it was being written.
fn building_files(dir: &Path) -> Vec<String> {
    read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".building"))
        .collect()
}

/// A small DPD, in the shape `stem-table` reads.
fn dpd(dir: &TempDir) -> PathBuf {
    let db = dir.child("dpd.db");
    let conn = Connection::open(&db).unwrap();
    conn.execute_batch(
        "CREATE TABLE dpd_headwords (id INTEGER, lemma_1 TEXT, stem TEXT);
        CREATE TABLE lookup (lookup_key TEXT, headwords TEXT);
        INSERT INTO dpd_headwords VALUES (1, 'gacchati 1', 'gacch'), (2, 'gata 1', 'ga');
        INSERT INTO lookup VALUES ('gacchati', '[1]'), ('gato', '[2]'), ('gatā', '[2]');",
    )
    .unwrap();
    db
}

#[test]
fn test_stem_table_builds_a_table_that_indexes_and_finds() {
    let dir = TempDir::new().unwrap();
    let (texts, db, stems) = (texts(&dir, r#"{"mn1:1": "gato"}"#), dpd(&dir), dir.child("stems.csv"));
    write(texts.join("mn2_root-pli-ms.json"), r#"{"mn2:1": "gatā"}"#).unwrap();
    succeeds(&pali(dir.path(), &["--texts", s(&texts), "--dpd-db", s(&db), "--stem-file", s(&stems), "stem-table"]));
    let table = read_to_string(&stems).unwrap();
    assert!(table.starts_with(STEM_HEADER), "{table}");
    assert!(table.lines().any(|line| line.starts_with("gato,ga,")), "{table}");
    assert_eq!(building_files(dir.path()), Vec::<String>::new());

    let index = dir.child("index");
    let args = ["--texts", s(&texts), "--stem-file", s(&stems), "--index-dir", s(&index), "-s", "dictionary"];
    succeeds(&pali(dir.path(), &[&args[..], &["index"]].concat()));
    // DPD gives gato and gatā one stem, so each finds the other's text.
    let found = succeeds(&pali(dir.path(), &["--stem-file", s(&stems), "--index-dir", s(&index), "find", "gato"]));
    assert!(found.contains("mn1") && found.contains("mn2"), "{found}");
}

#[test]
fn test_stem_table_without_the_dictionary_fails_and_creates_none() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "gato"}"#);
    let (db, stems) = (dir.child("dpd.db"), dir.child("stems.csv"));
    let error = fails(&pali(dir.path(), &["--texts", s(&texts), "--dpd-db", s(&db), "--stem-file", s(&stems), "stem-table"]));
    assert!(error.contains("unable to open database file"), "{error}");
    assert!(!db.exists(), "a missing DPD must not be created");
    assert!(!stems.exists());
}

#[test]
fn test_stem_table_without_texts_fails() {
    let dir = TempDir::new().unwrap();
    let db = dpd(&dir);
    let stems = dir.child("stems.csv");
    let missing = dir.child("none");
    let error = fails(&pali(dir.path(), &["--texts", s(&missing), "--dpd-db", s(&db), "--stem-file", s(&stems), "stem-table"]));
    assert!(error.contains("reading") && error.contains("none"), "{error}");
    let empty = dir.child("empty");
    create_dir(&empty).unwrap();
    let error = fails(&pali(dir.path(), &["--texts", s(&empty), "--dpd-db", s(&db), "--stem-file", s(&stems), "stem-table"]));
    assert!(error.contains("found no Pali texts"), "{error}");
    assert!(!stems.exists());
}

#[test]
fn test_stem_table_into_a_read_only_directory_fails_and_keeps_the_old_table() {
    let dir = TempDir::new().unwrap();
    let (texts, db) = (texts(&dir, r#"{"mn1:1": "gato"}"#), dpd(&dir));
    let locked = dir.child("locked");
    create_dir(&locked).unwrap();
    let stems = locked.join("stems.csv");
    write(&stems, STEMS).unwrap();
    set_permissions(&locked, PermissionsExt::from_mode(0o555)).unwrap();
    let output = pali(dir.path(), &["--texts", s(&texts), "--dpd-db", s(&db), "--stem-file", s(&stems), "stem-table"]);
    set_permissions(&locked, PermissionsExt::from_mode(0o755)).unwrap();
    let error = fails(&output);
    assert!(error.contains("creating") && error.contains(".building"), "{error}");
    assert_eq!(read_to_string(&stems).unwrap(), STEMS);
    assert_eq!(building_files(&locked), Vec::<String>::new());
}

#[test]
fn test_stem_table_onto_a_directory_fails_and_leaves_no_temporary_file() {
    let dir = TempDir::new().unwrap();
    let (texts, db) = (texts(&dir, r#"{"mn1:1": "gato"}"#), dpd(&dir));
    let occupied = dir.child("stems.csv");
    create_dir(&occupied).unwrap();
    write(occupied.join("thesis.txt"), "mine").unwrap();
    fails(&pali(dir.path(), &["--texts", s(&texts), "--dpd-db", s(&db), "--stem-file", s(&occupied), "stem-table"]));
    assert_eq!(read_to_string(occupied.join("thesis.txt")).unwrap(), "mine");
    assert_eq!(building_files(dir.path()), Vec::<String>::new());
}

/// The stems `stemmer` gives each line, as `stem-words` prints them.
fn stem_words(dir: &TempDir, stemmer: &str, input: &str) -> Vec<String> {
    let stems = dir.child("stems.csv");
    let output = succeeds(&pali_with_input(dir.path(), &["--stem-file", s(&stems), "-s", stemmer, "stem-words"], input));
    output
        .lines()
        .map(|line| line.split_once('\t').expect("a tab after the input").1.to_string())
        .collect()
}

#[test]
fn test_stem_words_reads_standard_input_with_each_stemmer() {
    let dir = TempDir::new().unwrap();
    stem_file(&dir, STEMS);
    let input = "dhammo\ngato\nnibba\u{0304}nassa\n";
    assert_eq!(stem_words(&dir, "snowball", input), ["dhamm", "gat", "nibb"]);
    assert_eq!(stem_words(&dir, "dictionary", input), ["dhamma", "gata", "nibbānassa"]);
}

#[test]
fn test_stem_words_composes_decomposed_input() {
    let dir = TempDir::new().unwrap();
    // A macron typed as a combining mark is one term, not two.
    assert_eq!(stem_words(&dir, "snowball", "nibba\u{0304}nassa\n"), stem_words(&dir, "snowball", "nibbānassa\n"));
    let output = succeeds(&pali_with_input(dir.path(), &["stem-words"], "nibba\u{0304}nassa\n"));
    // The line is echoed as given, and stemmed by snowball when no stemmer is named.
    assert_eq!(output, "nibba\u{0304}nassa\tnibb\n");
}

#[test]
fn test_stem_words_without_input_prints_nothing() {
    let dir = TempDir::new().unwrap();
    assert_eq!(succeeds(&pali(dir.path(), &["stem-words"])), "");
}

#[test]
fn test_a_stemmer_that_needs_a_table_refuses_a_bad_one() {
    let dir = TempDir::new().unwrap();
    for (contents, message) in [
        ("", "the stem table is empty; run stem-table again"),
        (STEM_HEADER, "the stem table is empty; run stem-table again"),
        ("term,dpd_stem,snowball_stem\ndhammo,dhamma\n", "found record with 2 fields"),
    ] {
        let stems = stem_file(&dir, contents);
        for command in [&["stem-words"][..], &["analyze", "dhammo"]] {
            let args = [&["--stem-file", s(&stems), "-s", "dictionary"][..], command].concat();
            let error = fails(&pali_with_input(dir.path(), &args, "dhammo\n"));
            assert!(error.contains(message), "{contents:?}: {error}");
        }
    }
    let missing = dir.child("none.csv");
    let error = fails(&pali(dir.path(), &["--stem-file", s(&missing), "-s", "dictionary", "stem-words"]));
    assert!(error.contains("reading stem file") && error.contains("none.csv"), "{error}");
}

#[test]
fn test_analyze_prints_the_index_terms() {
    let dir = TempDir::new().unwrap();
    let stems = stem_file(&dir, STEMS);
    let output = succeeds(&pali(dir.path(), &["--stem-file", s(&stems), "-s", "dictionary", "analyze", "Dhammo ca gato"]));
    // Lower-cased, the stop word ca dropped, each term looked up in the table.
    assert_eq!(output, "dhamma gata ");
    let output = succeeds(&pali(dir.path(), &["analyze", "nibba\u{0304}nassa"]));
    assert_eq!(output, "nibb ");
}

#[test]
fn test_pali_chars_lists_the_alphabet_in_order() {
    let dir = TempDir::new().unwrap();
    let output = succeeds(&pali(dir.path(), &["pali-chars"]));
    let chars: Vec<char> = output.split_whitespace().map(|c| c.chars().next().unwrap()).collect();
    for expected in ['a', 'ā', 'ṃ', 'ṁ', 'ñ', 'Ḍ'] {
        assert!(chars.contains(&expected), "{expected} in {output}");
    }
    assert!(!chars.contains(&'x') && !chars.contains(&'«'), "{output}");
    assert!(chars.is_sorted(), "{output}");
}

#[test]
fn test_other_chars_lists_what_the_tokenizer_drops() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "evaṁ «me» 1 x", "mn1:2": "sutaṁ"}"#);
    let output = succeeds(&pali(dir.path(), &["--texts", s(&texts), "other-chars"]));
    assert_eq!(output, "  1 x « » ");
}

#[test]
fn test_reports_on_the_texts_refuse_an_unreadable_one() {
    // A report on part of the texts must not pass for one on all of them, as index
    // and stem-table already refuse.
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "evaṃ «me» 1 x"}"#);
    write(texts.join("mn2_root-pli-ms.json"), "not json").unwrap();
    for command in ["other-chars", "stop-words"] {
        let error = fails(&pali(dir.path(), &["--texts", s(&texts), command]));
        assert!(error.contains("mn2_root-pli-ms.json"), "{command}: {error}");
    }
}

#[test]
fn test_a_reader_that_stops_early_is_not_an_error() {
    // As in `pali stem-words < forms.txt | head -1`.
    let dir = TempDir::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_pali"))
        .current_dir(dir.path())
        .args(["--stemmer", "snowball", "stem-words"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        for _ in 0..200_000 {
            if input.write_all(b"dhammassa gacchati\n").is_err() {
                break;
            }
        }
    });
    let mut first = [0u8; 9];
    std::io::Read::read_exact(child.stdout.as_mut().unwrap(), &mut first).unwrap();
    assert_eq!(&first, b"dhammassa");
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).is_empty(), "{}", stderr(&output));
}

#[test]
fn test_stop_words_lists_the_commonest_words() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "ca ca ca dhammo dhammo gato", "mn1:2": "ca"}"#);
    let output = succeeds(&pali(dir.path(), &["--texts", s(&texts), "stop-words", "-n", "2"]));
    assert_eq!(output, "ca\ndhammo\n");
    let output = succeeds(&pali(dir.path(), &["--texts", s(&texts), "stop-words"]));
    assert_eq!(output.lines().count(), 3);
}

#[test]
fn test_index_then_find_prints_the_matching_texts() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "evaṁ me sutaṁ", "mn1:2": "dhammo gato"}"#);
    write(texts.join("mn2_root-pli-ms.json"), r#"{"mn2:1": "nibbānaṁ"}"#).unwrap();
    // The default index directory, in the working directory.
    succeeds(&pali(dir.path(), &["--texts", s(&texts), "index"]));
    assert!(dir.child("index").join("stemmer.txt").exists());
    let found = succeeds(&pali(dir.path(), &["find", "dhammā"]));
    assert!(found.contains("mn1") && !found.contains("mn2"), "{found}");
    assert!(found.contains("Text UID") && found.contains("dhammo"), "{found}");
    let found = succeeds(&pali(dir.path(), &["find", "nibba\u{0304}naṁ"]));
    assert!(found.contains("mn2") && !found.contains("mn1"), "{found}");
    let found = succeeds(&pali(dir.path(), &["find", "xyzzy"]));
    assert!(!found.contains("mn"), "{found}");
    // A query that does not parse is searched as words, and the user told what was dropped.
    let output = pali(dir.path(), &["find", "dhammo", "NOT"]);
    assert!(succeeds(&output).contains("mn1"));
    assert!(stderr(&output).contains("ignored part of the query"), "{}", stderr(&output));
    // Nothing but stop words finds nothing, however it is written.
    for query in ["ca", "ca (", "AND ca"] {
        assert!(!succeeds(&pali(dir.path(), &["find", query])).contains("mn"), "{query}");
    }
}

#[test]
fn test_index_can_be_rebuilt_past_leftover_directories() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "dhammo"}"#);
    succeeds(&pali(dir.path(), &["--texts", s(&texts), "index"]));
    // Empty directories left by an interrupted run are cleared, not refused.
    create_dir(dir.child("index.building")).unwrap();
    create_dir(dir.child("index.previous")).unwrap();
    write(texts.join("mn1_root-pli-ms.json"), r#"{"mn1:1": "gato"}"#).unwrap();
    succeeds(&pali(dir.path(), &["--texts", s(&texts), "index"]));
    assert!(!dir.child("index.building").exists() && !dir.child("index.previous").exists());
    assert!(succeeds(&pali(dir.path(), &["find", "gato"])).contains("mn1"));
    assert!(!succeeds(&pali(dir.path(), &["find", "dhammo"])).contains("mn1"));
}

#[test]
fn test_index_without_texts_fails_and_keeps_the_old_index() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "dhammo"}"#);
    succeeds(&pali(dir.path(), &["--texts", s(&texts), "index"]));
    let error = fails(&pali(dir.path(), &["--texts", s(&dir.child("none")), "index"]));
    assert!(error.contains("no texts at"), "{error}");
    let empty = dir.child("empty");
    create_dir(&empty).unwrap();
    let error = fails(&pali(dir.path(), &["--texts", s(&empty), "index"]));
    assert!(error.contains("found no Pali texts to index"), "{error}");
    assert!(!dir.child("index.building").exists());
    assert!(succeeds(&pali(dir.path(), &["find", "dhammo"])).contains("mn1"));
}

#[test]
fn test_a_rebuild_with_a_bad_stem_table_keeps_the_old_index() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "dhammo"}"#);
    let stems = stem_file(&dir, STEMS);
    let args = ["--texts", s(&texts), "--stem-file", s(&stems), "-s", "dictionary"];
    succeeds(&pali(dir.path(), &[&args[..], &["index"]].concat()));
    stem_file(&dir, STEM_HEADER);
    let error = fails(&pali(dir.path(), &[&args[..], &["index"]].concat()));
    assert!(error.contains("run stem-table again"), "{error}");
    // The table is refused before anything is removed, so the old index still answers.
    stem_file(&dir, STEMS);
    assert!(succeeds(&pali(dir.path(), &["--stem-file", s(&stems), "find", "dhammo"])).contains("mn1"));
}

#[test]
fn test_index_will_not_replace_another_directory() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "dhammo"}"#);
    let precious = dir.child("precious");
    create_dir(&precious).unwrap();
    write(precious.join("thesis.txt"), "years of work").unwrap();
    let error = fails(&pali(dir.path(), &["--texts", s(&texts), "--index-dir", s(&precious), "index"]));
    assert!(error.contains("will not be replaced"), "{error}");
    assert_eq!(read_to_string(precious.join("thesis.txt")).unwrap(), "years of work");
    assert_eq!(read_dir(&precious).unwrap().count(), 1);
    assert!(!dir.child("precious.building").exists());
}

#[test]
fn test_find_refuses_another_stemmer() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "dhammo"}"#);
    let stems = stem_file(&dir, STEMS);
    succeeds(&pali(dir.path(), &["--texts", s(&texts), "index"]));
    let error = fails(&pali(dir.path(), &["--stem-file", s(&stems), "-s", "dictionary", "find", "dhammo"]));
    assert!(error.contains("built with --stemmer snowball"), "{error}");
    // Naming the stemmer it was built with is fine.
    assert!(succeeds(&pali(dir.path(), &["-s", "snowball", "find", "dhammo"])).contains("mn1"));
}

#[test]
fn test_find_refuses_an_index_whose_stem_table_changed() {
    let dir = TempDir::new().unwrap();
    let texts = texts(&dir, r#"{"mn1:1": "dhammo"}"#);
    let stems = stem_file(&dir, STEMS);
    succeeds(&pali(dir.path(), &["--texts", s(&texts), "--stem-file", s(&stems), "-s", "dictionary", "index"]));
    assert!(succeeds(&pali(dir.path(), &["--stem-file", s(&stems), "find", "dhammo"])).contains("mn1"));
    stem_file(&dir, &format!("{STEMS}dhammassa,dhamma,dhamm\n"));
    let error = fails(&pali(dir.path(), &["--stem-file", s(&stems), "find", "dhammo"]));
    assert!(error.contains("stem table have changed since this index was built"), "{error}");
}

#[test]
fn test_find_without_an_index_fails() {
    let dir = TempDir::new().unwrap();
    let error = fails(&pali(dir.path(), &["find", "dhammo"]));
    assert!(error.contains("has no record of its stemmer"), "{error}");
}

#[test]
fn test_a_bad_command_line_is_refused() {
    let dir = TempDir::new().unwrap();
    for args in [&[][..], &["no-such-command"], &["-s", "porter", "analyze", "x"], &["analyze"]] {
        let error = fails(&pali(dir.path(), args));
        assert!(error.contains("Usage") || error.contains("error"), "{args:?}: {error}");
    }
}
