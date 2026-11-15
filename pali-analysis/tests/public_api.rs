//! The library's public boundary: opening DPD, resolving terms against it, and the
//! analyzers' token streams. Kept outside the analyzer sources, whose bytes feed the
//! index fingerprint.

use pali_analysis::analyzers::AnalyzerConfig;
use pali_analysis::dict_stemmer::DictionaryMode;
use pali_analysis::dpd::Dictionary;
use rusqlite::Connection;
use std::fs::{read, set_permissions, write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tantivy::tokenizer::{TextAnalyzer, TokenStream};
use temp_dir::TempDir;

const SCHEMA: &str = "CREATE TABLE dpd_headwords (id INTEGER, lemma_1 TEXT, pos TEXT, root_key TEXT,
        family_root TEXT, root_base TEXT, construction TEXT);
    CREATE TABLE lookup (lookup_key TEXT, headwords TEXT, deconstructor TEXT);";

fn database(path: &Path, rows: &str) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(&format!("{SCHEMA}{rows}")).unwrap();
}

fn terms(words: &[&str]) -> Vec<(String, u64)> {
    words.iter().map(|word| ((*word).to_string(), 1)).collect()
}

#[test]
fn test_opening_a_missing_dictionary_fails_and_creates_nothing() {
    let dir = TempDir::new().unwrap();
    let missing = dir.child("dpd.db");
    let error = Dictionary::open(&missing).unwrap_err();
    assert!(error.to_string().contains("unable to open database file"), "{error}");
    assert!(!missing.exists());
}

#[test]
fn test_a_read_only_dictionary_opens_and_is_not_written() {
    let dir = TempDir::new().unwrap();
    let db = dir.child("dpd.db");
    database(
        &db,
        "INSERT INTO dpd_headwords VALUES (1, 'gata 1', 'pp', '√gam 1', '√gam', '', '');
        INSERT INTO lookup VALUES ('gato', '[1]', '');",
    );
    let before = read(&db).unwrap();
    set_permissions(&db, PermissionsExt::from_mode(0o444)).unwrap();
    let dictionary = Dictionary::open(&db).unwrap();
    assert_eq!(dictionary.lemmas().unwrap(), ["gata"]);
    let entries = dictionary.entries(&terms(&["gato", "xyzzy"])).unwrap();
    assert_eq!(entries["gato"].word.as_deref(), Some("gata"));
    assert_eq!(entries["xyzzy"].word, None);
    drop(dictionary);
    assert_eq!(read(&db).unwrap(), before);
}

#[test]
fn test_a_file_that_is_not_a_dictionary_is_refused() {
    let dir = TempDir::new().unwrap();
    let db = dir.child("dpd.db");
    write(&db, "not a database at all, just some text that is long enough").unwrap();
    let opened = Dictionary::open(&db);
    // SQLite opens lazily, so the refusal may come on the first query instead.
    assert!(opened.is_err() || opened.unwrap().lemmas().is_err());
    let empty = dir.child("empty.db");
    Connection::open(&empty).unwrap().execute_batch("CREATE TABLE unrelated (x);").unwrap();
    let dictionary = Dictionary::open(&empty).unwrap();
    assert!(dictionary.lemmas().unwrap_err().to_string().contains("no such table"));
    assert!(dictionary.check_preferred_roots().is_err());
    assert!(dictionary.entries(&terms(&["gato"])).is_err());
}

#[test]
fn test_a_lookup_naming_a_missing_headword_is_an_error() {
    let dir = TempDir::new().unwrap();
    let db = dir.child("dpd.db");
    database(
        &db,
        "INSERT INTO dpd_headwords VALUES (1, 'gata 1', 'pp', '√gam 1', '√gam', '', '');
        INSERT INTO lookup VALUES ('gato', '[7]', '');",
    );
    let error = Dictionary::open(&db).unwrap().entries(&terms(&["gato"])).unwrap_err();
    assert!(format!("{error:#}").contains("headword 7 for gato, which does not exist"), "{error:#}");
}

#[test]
fn test_a_lengthened_final_u_falls_back_to_its_pre_sandhi_spelling() {
    let dir = TempDir::new().unwrap();
    let db = dir.child("dpd.db");
    // A headword with no lemma at all must not stop the others resolving.
    database(
        &db,
        "INSERT INTO dpd_headwords VALUES (1, 'bahu 1', 'adj', '', '', '', ''), (2, '', 'ind', '', '', '', '');
        INSERT INTO lookup VALUES ('bahu', '[1]', '');",
    );
    let dictionary = Dictionary::open(&db).unwrap();
    let entries = dictionary.entries(&terms(&["bahū", "bahu"])).unwrap();
    assert_eq!(entries["bahū"].word.as_deref(), Some("bahu"));
    assert_eq!(entries["bahu"].word.as_deref(), Some("bahu"));
    assert_eq!(dictionary.lemmas().unwrap(), ["bahu", ""]);
}

#[test]
fn test_preferred_roots_must_all_be_present() {
    let dir = TempDir::new().unwrap();
    let db = dir.child("dpd.db");
    database(&db, "INSERT INTO dpd_headwords VALUES (1, 'gacchati 1', 'pr', '√gam 1', '√gam', '', '');");
    let error = Dictionary::open(&db).unwrap().check_preferred_roots().unwrap_err();
    assert!(error.to_string().contains("update PREFERRED_ROOTS"), "{error}");
}

/// The tokens of `text`, after changing each token in place through `token_mut`.
fn mutated_tokens(config: AnalyzerConfig, text: &str) -> Vec<String> {
    let mut analyzer = TextAnalyzer::try_from(config).unwrap();
    let mut stream = analyzer.token_stream(text);
    let mut tokens = Vec::new();
    while stream.advance() {
        stream.token_mut().text.push('!');
        tokens.push(stream.token().text.clone());
    }
    tokens
}

#[test]
fn test_stemmed_tokens_can_be_changed_in_place() {
    assert_eq!(mutated_tokens(AnalyzerConfig::Algorithmic, "dhammo gato"), ["dhamm!", "gat!"]);
    let stem_table = b"term,dpd_stem,snowball_stem,family,fallback\ndhammo,dhamma,dhamm,,dhamma\n".to_vec();
    let config = AnalyzerConfig::Dictionary { stem_table, mode: DictionaryMode::Plain };
    assert_eq!(mutated_tokens(config, "dhammo gato"), ["dhamma!", "gato!"]);
}

#[test]
fn test_a_dictionary_analyzer_needs_a_valid_table() {
    let config = AnalyzerConfig::Dictionary { stem_table: Vec::new(), mode: DictionaryMode::Hybrid };
    let error = TextAnalyzer::try_from(config).err().expect("an empty table is refused");
    assert!(error.to_string().contains("no term column"), "{error}");
}
