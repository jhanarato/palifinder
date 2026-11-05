//! Inflections the texts never use: a searcher can type one, and it must find its own
//! word, without changing any row the texts and headwords give the table.

use pali_analysis::dpd::Dictionary;
use pali_analysis::table::{StemTable, TermStem};
use pali_analysis::tokenizer::PaliTokenizer;
use pali_analysis::vocabulary::Vocabulary;
use rusqlite::Connection;
use std::collections::HashMap;
use tantivy::tokenizer::TextAnalyzer;

const SCHEMA: &str = "CREATE TABLE dpd_headwords (id INTEGER, lemma_1 TEXT, pos TEXT, root_key TEXT,
        family_root TEXT, root_base TEXT, construction TEXT);
    CREATE TABLE lookup (lookup_key TEXT, headwords TEXT, deconstructor TEXT);";

/// gacchati "goes" and its causative gameti "causes to go", with futures the texts
/// below never use, as DPD lists them.
const GOING: &str = "INSERT INTO dpd_headwords VALUES
        (1, 'gacchati 1', 'pr', '√gam 1', '√gam', '', ''),
        (2, 'gameti 1', 'pr', '√gam 1', '√gam', '√gam + e (caus)', ''),
        (3, 'hanu 1', 'fem', '', '', '', ''),
        (4, 'hanūna', 'sandhi', '', '', '', '');
    INSERT INTO lookup VALUES ('gacchati', '[1]', ''), ('gacchanti', '[1]', ''),
        ('gamissati', '[1, 2]', ''), ('gameti', '[2]', ''), ('gamessati', '[2]', ''),
        ('hanūni', '[3]', ''), ('yaṃ kiñci', '[1]', ''), ('dukkhaṃ', '', 'dukkha + ṃ');";

fn table(texts: &str, rows: &str) -> HashMap<String, TermStem> {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(&format!("{SCHEMA}{rows}")).unwrap();
    let analyzer = TextAnalyzer::builder(PaliTokenizer::default()).build();
    let table = StemTable::new(Vocabulary::new([texts], analyzer), &Dictionary::from(conn)).unwrap();
    table.into_iter().map(|record| (record.term.clone(), record)).collect()
}

#[test]
fn test_an_unseen_causative_future_finds_the_causative() {
    let rows = table("gacchati gacchanti gameti", GOING);
    let gamessati = &rows["gamessati"];
    assert_eq!(gamessati.dpd_stem.as_deref(), Some("gameti"));
    assert_eq!(gamessati.family.as_deref(), Some("family:√gam|√gam 1|caus"));
    assert_eq!(gamessati.family, rows["gameti"].family);
}

#[test]
fn test_ordinary_and_causative_verbs_stay_apart() {
    let rows = table("gacchati gacchanti gameti", GOING);
    assert_eq!(rows["gacchati"].family.as_deref(), Some("family:√gam|√gam 1|"));
    assert_ne!(rows["gamessati"].family, rows["gacchati"].family);
    assert_ne!(rows["gameti"].family, rows["gacchanti"].family);
}

#[test]
fn test_only_single_words_with_a_headword_are_added() {
    let rows = table("gacchati", GOING);
    assert!(rows.contains_key("gamessati"));
    // A phrase the tokenizer can never produce, and a form DPD only breaks down.
    assert!(!rows.keys().any(|term| term.contains(' ')));
    assert!(!rows.contains_key("dukkhaṁ"));
}

#[test]
fn test_an_unseen_form_is_spelled_as_the_texts_spell_it() {
    let rows = table("gacchati", "INSERT INTO dpd_headwords VALUES (1, 'dukkha 1', 'nt', '', '', '', '');
        INSERT INTO lookup VALUES ('dukkhaṃ', '[1]', '');");
    assert_eq!(rows["dukkhaṁ"].dpd_stem.as_deref(), Some("dukkha"));
    assert!(!rows.contains_key("dukkhaṃ"));
}

#[test]
fn test_unseen_forms_change_no_row_the_texts_give() {
    // hanūna is a headword (a sandhi) with no word of its own, which the texts never
    // use, and it shares its Snowball stem with the unseen inflection hanūni. Counted
    // among the fallbacks, hanūni's word would become hanūna's fallback.
    let texts = "gacchati gacchanti gameti";
    let without = table(texts, &GOING.replace("('hanūni', '[3]', ''), ", "").replace("('gamessati', '[2]', ''), ", "")
        .replace("('gamissati', '[1, 2]', ''), ", ""));
    let with = table(texts, GOING);
    for (term, row) in &without {
        assert_eq!(&with[term], row, "{term}");
    }
    assert_eq!(with["hanūna"].dpd_stem, None);
    assert_eq!(with["hanūna"].fallback, None);
    assert_eq!(with["hanūni"].dpd_stem.as_deref(), Some("hanu"));
}
