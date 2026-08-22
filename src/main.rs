pub mod dpd;
pub mod texts;
pub mod commands;
pub mod table;
pub mod tokenizer;
pub mod vocabulary;

use crate::dpd::stem;
use crate::texts::PaliFiles;
use anyhow::Result;
use clap::Parser;
use commands::{Arguments, Command};
use rusqlite::Connection;

fn main() -> Result<()> {
    let args = Arguments::parse();
    match args.command {
        Command::StemTable {
            texts,
            stem_file,
        } => {
            let files = PaliFiles::new(texts);
            let mut conn = Connection::open(args.dpd_db.as_path())?;
            let table = table::stem_table(&mut conn, &files.vocabulary()?);
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
