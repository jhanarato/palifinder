use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Arguments {
    #[arg(
        long = "dpd-db",
        default_value = "data/dpd.db",
        help = "Digital pali dictionary SQLite database file"
    )]
    pub dpd_db: PathBuf,

    #[arg(
        long = "texts",
        default_value = "/opt/sc/sc-flask/sc-data/sc_bilara_data/root/pli/ms",
        help = "Directory containing Pali root texts"
    )]
    pub texts: PathBuf,

    #[arg(
        long = "stem-file",
        default_value = "data/stems.csv",
        help = "Location of file containing stems"
    )]
    pub stem_file: PathBuf,
    
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Display the possible stems of a given term
    Stem {
        #[arg(help = "The term to stem")]
        term: String,
    },
    /// Show headword keys for term lookup.
    DpdLookup {
        #[arg(help = "Term to look for")]
        term: String,
    },
    /// Save the lemmas and stems for all texts.
    StemTable,
    /// Show charcters recognised as Pali
    PaliChars,
    /// Display all non-Pali characters
    OtherChars,
}
