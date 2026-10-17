use clap::{Parser, Subcommand, ValueEnum};
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

    #[arg(
        short = 's',
        long = "stemmer",
        default_value = "snowball",
        help = "Stemmer variant for text analyzer"
    )]
    pub stemmer: Stemmer,

    #[arg(
        long = "index-dir",
        default_value = "index",
        help = "Location of index"
    )]
    pub index_path: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create the index
    Index,
    /// Search the index
    Find {
        #[arg(trailing_var_arg = true)]
        query: Vec<String>,
    },
    /// Display the tokens emitted by the text analyzer
    Analyze {
        #[arg(help = "The text in quotes")]
        text: String,
    },
    /// Create the CSV stem file
    StemTable,
    /// Display Pali characters expected by the tokenizer
    PaliChars,
    /// Display all characters excluded by the tokenizer
    OtherChars,
    /// List terms by frequency, descending
    StopWords {
        #[arg(
            short = 'n',
            long = "number",
            default_value_t = 10,
            help = "How many of the most frequent words to display"
        )]
        number: usize,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum Stemmer {
    /// Use snowball algorithm
    Snowball,
    /// Use dictionary lookup
    Dictionary,
}
