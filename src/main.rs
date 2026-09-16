mod algo_stemmer;
pub mod commands;
mod dict_stemmer;
pub mod dpd;
pub mod snowball;
pub mod table;
#[cfg(test)]
pub mod tests;
pub mod texts;
pub mod tokenizer;
pub mod vocabulary;
mod stop_words;
pub mod analyzers;

use crate::analyzers::Analyzer;
use crate::dpd::Dictionary;
use crate::stop_words::most_frequent_words;
use crate::table::StemTable;
use crate::texts::PaliFiles;
use crate::tokenizer::PaliTokenizer;
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use clap::Parser;
use commands::{Arguments, Command};
use csv::Reader;
use rusqlite::Connection;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tantivy::tokenizer::{LowerCaser, TextAnalyzer, Token, TokenStream};

fn main() -> Result<()> {
    let args = Arguments::parse();
    match args.command {
        Command::StemTable => {
            create_stem_table(&args.texts, &args.dpd_db, &args.stem_file)?;
        }
        Command::Analyze { algorithmic, text } => {
            analyze_text(algorithmic, text.as_str(), &args.stem_file)?;
        }
        Command::DpdLookup { term } => {
            lookup_term_in_dictionary(term.as_str(), args.dpd_db.as_path())?;
        }
        Command::PaliChars => {
            show_pali_characters();
        }
        Command::OtherChars => {
            show_other_characters(args.texts);
        }
        Command::StopWords { number } => {
            show_stop_words(args.texts, number);
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
        .build();
    let vocabulary = Vocabulary::new(files.segments(), analyzer);
    let conn = Connection::open(dictionary_path)?;
    let dictionary = Dictionary::from(conn);
    let term_stems = StemTable::new(vocabulary, &dictionary);
    term_stems.save(stem_file_path)?;
    Ok(())
}

fn analyze_text(algorithmic: bool, text: &str, stem_file_path: &Path) -> Result<()> {
    let mut analyzer = if algorithmic {
        Analyzer::StemAlgorithmic.build()
    } else {
        let reader = Reader::from_path(stem_file_path)?;
        let table = StemTable::try_from(reader)?;
        Analyzer::StemDictionary { table }.build()
    };
    let mut token_stream = analyzer.token_stream(text);
    token_stream.process(&mut |token: &Token| print!("{0} ", token.text));
    Ok(())
}

fn lookup_term_in_dictionary(term: &str, dictionary_path: &Path) -> Result<()> {
    let conn = Connection::open(dictionary_path)?;
    let dict = Dictionary::from(conn);
    let ids = dict.lookup(term);
    match ids {
        Err(e) => println!("An error occured: {e:#?}"),
        Ok(ids) if ids.is_empty() => println!("Nothing found"),
        Ok(ids) => ids.iter().for_each(|id| println!("{id}")),
    }
    Ok(())
}

fn show_pali_characters() {
    let tokenizer = PaliTokenizer::default();
    let mut alphabet: Vec<char> = tokenizer.alphabet.into_iter().collect();
    alphabet.sort_unstable();
    for char in alphabet {
        print!("{char} ");
    }
}

fn show_other_characters(texts_path: PathBuf) {
    let files = PaliFiles::new(texts_path);
    let mut chars = BTreeSet::<char>::new();
    for segment in files.segments() {
        for char in segment.text.chars() {
            chars.insert(char);
        }
    }
    let tokenizer = PaliTokenizer::default();
    for char in chars {
        if !tokenizer.alphabet.contains(&char) {
            print!("{char} ");
        }
    }
}

fn show_stop_words(texts_path: PathBuf, number: usize) {
    let files = PaliFiles::new(texts_path);
    for word in most_frequent_words(files.segments(), number) {
        println!("{word}");
    }

}