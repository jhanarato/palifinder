pub mod dpd;
pub mod texts;

use anyhow::Result;
use clap::{Parser, Subcommand};
use csv::Writer;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tantivy::tokenizer::{TokenStream, Tokenizer, WhitespaceTokenizer};
use crate::dpd::stem;

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

fn stem_table(
    conn: &mut Connection,
    vocabulary: &HashSet<String>,
) -> HashMap<String, Option<String>> {
    let mut table: HashMap<String, Option<String>> = HashMap::new();
    for term in vocabulary {
        let term_stem = dpd::stem(conn, term);
        match term_stem {
            Ok(stem) => {
                table.insert(term.clone(), Some(stem));
            }
            Err(_) => {
                table.insert(term.clone(), None);
            }
        }
    }
    table
}

#[derive(Serialize)]
struct TermStem {
    term: String,
    stem: Option<String>,
}

fn save_table(table: &HashMap<String, Option<String>>, path: &Path) -> Result<()> {
    let mut writer = Writer::from_path(path)?;
    for (key, value) in table {
        let term_stem = TermStem {
            term: key.clone(),
            stem: value.clone(),
        };
        writer.serialize(term_stem)?;
    }
    Ok(())
}

#[derive(Parser, Debug)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Stem {
        #[arg(
            long = "dpd-db",
            default_value = "data/dpd.db",
            help = "Digital pali dictionary SQLite database file"
        )]
        dpd_db: PathBuf,
        #[arg(help = "The Pali word to stem")]
        word: String,
    },
    StemTable {
        #[arg(
            long = "texts",
            default_value = "/opt/sc/sc-flask/sc-data/sc_bilara_data/root/pli/ms",
            help = "Directory containing Pali root texts"
        )]
        texts: PathBuf,
        #[arg(
            long = "dpd-db",
            default_value = "data/dpd.db",
            help = "Digital pali dictionary SQLite database file"
        )]
        dpd_db: PathBuf,
        #[arg(
            long = "stem-file",
            default_value = "data/stems.csv",
            help = "Location of output file",
        )]
        stem_file: PathBuf,
    },
}

fn main() -> Result<()> {
    let args = Arguments::parse();
    match args.command {
        Command::StemTable {
            texts,
            dpd_db,
            stem_file,
        } => {
            let vocabulary = vocabulary(texts.as_path())?;
            let mut conn = Connection::open(dpd_db.as_path())?;
            let table = stem_table(&mut conn, &vocabulary);
            save_table(&table, stem_file.as_path())?;
        },
        Command::Stem {dpd_db, word} => {
            let mut conn = Connection::open(dpd_db.as_path())?;
            let stem = stem(&mut conn, word.as_str());
            match stem {
                Ok(stem) => println!("{stem}"),
                Err(_) => println!("Stem not found"),
            }

        }
    }
    Ok(())
}
