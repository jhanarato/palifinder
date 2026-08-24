pub mod commands;
pub mod dpd;
pub mod table;
pub mod texts;
pub mod tokenizer;
pub mod vocabulary;

use crate::dpd::stem;
use crate::texts::PaliFiles;
use crate::tokenizer::{PaliChars, PaliTokenizer};
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use clap::Parser;
use commands::{Arguments, Command};
use rusqlite::Connection;
use std::collections::BTreeSet;

fn main() -> Result<()> {
    let args = Arguments::parse();
    match args.command {
        Command::StemTable { stem_file } => {
            let files = PaliFiles::new(args.texts);
            let vocabulary = Vocabulary::new(files.segments(), PaliTokenizer::default());
            let mut conn = Connection::open(args.dpd_db.as_path())?;
            let table = table::stem_table(&mut conn, vocabulary);
            table::save_table(&table, stem_file.as_path())?;
        }
        Command::Stem { word } => {
            let mut conn = Connection::open(args.dpd_db.as_path())?;
            let stem = stem(&mut conn, word.as_str());
            match stem {
                Ok(stem) => println!("{stem}"),
                Err(_) => println!("Stem not found"),
            }
        }
        Command::Chars {} => {
            let files = PaliFiles::new(args.texts);
            let mut chars = BTreeSet::<char>::new();
            for segment in files.segments() {
                for char in segment.text.chars() {
                    chars.insert(char);
                }
            }
            for char in chars {
                print!("{char} ");
            }
        }
        Command::SplitOn {} => {
            let files = PaliFiles::new(args.texts);
            let mut chars = BTreeSet::<char>::new();
            for segment in files.segments() {
                for char in segment.text.chars() {
                    chars.insert(char);
                }
            }
            let pali_chars = PaliChars::default();
            for char in chars {
                if !pali_chars.is_pali(char) {
                    print!("{char} ");
                }
            }
        }
    }
    Ok(())
}
