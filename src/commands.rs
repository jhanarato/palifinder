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

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Print the stem of a Pali word
    Stem {
        #[arg(help = "The Pali word to stem")]
        word: String,
    },
    /// Save the lemmas and stems for all texts.
    StemTable {
        #[arg(
            long = "stem-file",
            default_value = "data/stems.csv",
            help = "Location of output file"
        )]
        stem_file: PathBuf,
    },
    /// Display all characters found in the texts
    Chars {},
}
