pub mod dpd;
pub mod texts;
pub mod commands;
pub mod table;

use crate::dpd::stem;
use anyhow::Result;
use clap::Parser;
use commands::{Arguments, Command};
use rusqlite::Connection;
use std::collections::HashSet;
use std::path::Path;
use tantivy::tokenizer::{TokenStream, Tokenizer, WhitespaceTokenizer};

fn tokenize(segment: &str) -> Vec<String> {
    let mut tokenizer = WhitespaceTokenizer::default();
    let mut stream = tokenizer.token_stream(segment);
    let mut tokens = Vec::new();
    stream.process(&mut |token| {
        tokens.push(token.text.clone());
    });

    tokens
}

fn vocabulary(pali_dir: &Path) -> Result<HashSet<String>> {
    let mut vocabulary: HashSet<String> = HashSet::new();
    for file in texts::pali_files(pali_dir) {
        let contents = std::fs::read_to_string(file)?;
        for segment in texts::segments(contents.as_str())? {
            for token in tokenize(segment.as_str()) {
                vocabulary.insert(token);
            }
        }
    }
    Ok(vocabulary)
}

fn main() -> Result<()> {
    let args = Arguments::parse();
    match args.command {
        Command::StemTable {
            texts,
            stem_file,
        } => {
            let vocabulary = vocabulary(texts.as_path())?;
            let mut conn = Connection::open(args.dpd_db.as_path())?;
            let table = table::stem_table(&mut conn, &vocabulary);
            table::save_table(&table, stem_file.as_path())?;
        },
        Command::Stem {word} => {
            let mut conn = Connection::open(args.dpd_db.as_path())?;
            let stem = stem(&mut conn, word.as_str());
            match stem {
                Ok(stem) => println!("{stem}"),
                Err(_) => println!("Stem not found"),
            }

        }
    }
    Ok(())
}
