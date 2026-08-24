pub mod commands;
pub mod dpd;
pub mod table;
pub mod texts;
pub mod tokenizer;
pub mod vocabulary;

use crate::dpd::stem;
use crate::texts::{PaliFiles, Segment};
use crate::tokenizer::PaliTokenizer;
use crate::vocabulary::Vocabulary;
use anyhow::Result;
use clap::Parser;
use commands::{Arguments, Command};
use rusqlite::Connection;

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
            let segments: Vec<Segment> = files.segments().collect();
            println!("Segment count: {}", segments.len());
        }
    }
    Ok(())
}
