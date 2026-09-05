mod algo_stemmer;
pub mod commands;
mod dict_stemmer;
pub mod dpd;
#[allow(clippy::all)]
pub mod snowball;
pub mod table;
#[cfg(test)]
pub mod tests;
pub mod texts;
pub mod tokenizer;
pub mod vocabulary;

use crate::dict_stemmer::DictionaryStemmer;
use crate::dpd::Dictionary;
use crate::table::TermStems;
use crate::texts::PaliFiles;
use crate::tokenizer::PaliTokenizer;
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use clap::Parser;
use commands::{Arguments, Command};
use csv::Reader;
use rusqlite::Connection;
use std::collections::BTreeSet;
use tantivy::tokenizer::{LowerCaser, TextAnalyzer, Token, TokenStream};

fn main() -> Result<()> {
    let args = Arguments::parse();
    match args.command {
        Command::StemTable => {
            let files = PaliFiles::new(args.texts);
            let analyzer = TextAnalyzer::builder(PaliTokenizer::default())
                .filter(LowerCaser)
                .build();
            let vocabulary = Vocabulary::new(files.segments(), analyzer);
            let conn = Connection::open(args.dpd_db.as_path())?;
            let dictionary = Dictionary::from(conn);
            let term_stems = TermStems::new(vocabulary, &dictionary);
            term_stems.save(&args.stem_file)?;
        }
        Command::Analyze { algorithmic, text } => {
            let mut analyzer = if algorithmic {
                let stemmer = algo_stemmer::Stemmer {};
                TextAnalyzer::builder(PaliTokenizer::default())
                    .filter(LowerCaser)
                    .filter(stemmer)
                    .build()
            } else {
                let reader = Reader::from_path(args.stem_file)?;
                let stemmer = DictionaryStemmer::try_from(reader)?;
                TextAnalyzer::builder(PaliTokenizer::default())
                    .filter(LowerCaser)
                    .filter(stemmer)
                    .build()

            };
            let mut token_stream = analyzer.token_stream(text.as_str());
            token_stream.process(&mut |token: &Token| println!("{0}", token.text));
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
