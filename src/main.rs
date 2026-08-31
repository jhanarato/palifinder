pub mod commands;
pub mod dpd;
pub mod table;
pub mod texts;
pub mod tokenizer;
pub mod vocabulary;
mod dict_stemmer;
#[cfg(test)]
pub mod tests;

use crate::dpd::Dictionary;
use crate::table::TermStems;
use crate::texts::PaliFiles;
use crate::tokenizer::PaliTokenizer;
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use clap::Parser;
use commands::{Arguments, Command};
use rusqlite::Connection;
use std::collections::BTreeSet;
use tantivy::tokenizer::{LowerCaser, TextAnalyzer};

fn main() -> Result<()> {
    let args = Arguments::parse();
    match args.command {
        Command::StemTable { stem_file } => {
            let files = PaliFiles::new(args.texts);
            let analyzer = TextAnalyzer::builder(PaliTokenizer::default())
                .filter(LowerCaser)
                .build();
            let vocabulary = Vocabulary::new(files.segments(), analyzer);
            let conn = Connection::open(args.dpd_db.as_path())?;
            let dictionary = Dictionary::from(conn);
            let term_stems = TermStems::new(vocabulary, &dictionary);
            term_stems.save(&stem_file)?;
        }
        Command::Stem { term } => {
            let conn = Connection::open(args.dpd_db.as_path())?;
            let dict = Dictionary::from(conn);
            let stems = dict.stems(term.as_str());
            match stems {
                Err(e) => println!("An error occured: {e:#?}"),
                Ok(stems) if stems.is_empty() => println!("No stem found"),
                Ok(stems) => stems.iter().for_each(|stem| println!("{stem}")),
            }
        }
        Command::DpdLookup { term } => {
            let conn = Connection::open(args.dpd_db.as_path())?;
            let dict = Dictionary::from(conn);
            let ids = dict.lookup(term.as_str());
            match ids {
                Err(e) => println!("An error occured: {e:#?}"),
                Ok(ids) if ids.is_empty() => println!("Nothing found"),
                Ok(ids) => ids.iter().for_each(|id| println!("{id}")),
            }
        }
        Command::PaliChars => {
            let tokenizer = PaliTokenizer::default();
            let mut alphabet: Vec<char> = tokenizer.alphabet.into_iter().collect();
            alphabet.sort_unstable();
            for char in alphabet {
                print!("{char} ");
            }
        }
        Command::OtherChars => {
            let files = PaliFiles::new(args.texts);
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
    }
    Ok(())
}
