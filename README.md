# Pālifinder

Pālifinder is a command line search engine for the Pāli Canon, written in Rust and using the [Tantivy](https://github.com/quickwit-oss/tantivy) crate.

<img width="1859" height="572" alt="palifinder" src="https://github.com/user-attachments/assets/e87ab2b2-c1e2-4016-99dc-70893cef0e4a" />

Tantivy is comparable to [Apache Lucene](https://lucene.apache.org/), the indexing engine used by ElasticSearch. 

For the most part Pālifinder should be considered a prototype, though it is certainly usable and hopefully useful. It includes:

- A tokenizer that recognizes Pāli, as encoded in SuttaCentral's segmented Pāli texts.
- Two stemmers: one using dictionary lookup and the other compiled from Snowball to pure Rust.
- A stopword filter.
- Word lookup for the Digital Pali Dictionary.
- Indexing of SuttaCentral's segmented Pāli texts.
- Search queries with keywords-in-context display.  

As a prototype, it has limitations:

- The Pāli language only, not the translated texts.
- Command line only. No REST API.
- The dictionary stemmer has stems for about half the vocabulary. This could be improved with better database queries.
- The Snowball stemmer stems the entire vocabulary but the algorithm is incomplete.
- Default Tantivy query parser. [There's a lot more in the Tantivy toolchest](https://docs.rs/tantivy/latest/tantivy/query/index.html).

## Why two stemmers?

We have two competing stemmers. Our algorithmic stemmer was written in Snowball by Ven. Khemarato and compiled to Rust. The algorithm is still a work in progress. The Pālifinder lets us test the algorithm and get it ready for production work. With Snowball's portability it can be easily integrated into any number of search backends.  [ElasticSearch for instance](https://www.elastic.co/docs/reference/text-analysis/analysis-snowball-tokenfilter).

The dictionary stemmer stems (pun intended) from a conversation with the [Digital Pāli Dictionary](https://dpdict.net/) proponents. Given that the hard work has already been done to match a stem to every word in the dictionary, we should be able to extract the stems for each word, store it as a map in memory and use the map as a lookup table for the stemmer. In practice, the DPD database is unnormalized. I was only able to extract a stem for about half the terms in SuttaCentral's Pāli. I'm sure this can be improved upon by those more familiar with Pāli and data extraction generally. 

The other issue I wanted to address was how much memory the map would use. As it turns out we're looking at about 5MB of data on disk, so a trivial amount of space given today's resources. The speed of the lookup is pretty similar, though the Snowball implementation may become slower as it gets more complex.

Really there's no reason not to persue both.

## Installation & setup

As this is a prototype, there are no releases. To build and install:

- Install the Rust toolchain for your platform: https://rustup.rs/
- In the source directory run `cargo install`

### Dictionary stemmer

By default, Pālifinder uses the included Snowball stemmer. If you want to use or test the dictionary stemmer, you'll need the SQLite database of the Digital Pāli Dictionary. 

It can be downloaded here:

https://github.com/digitalpalidictionary/dpd-db/releases/download/v0.4.20260728/dpd.db.tar.xz

Unzip into the `data/` directory or another of your choice. The database location can be specified at the command line.

### Download the Pāli JSON files for indexing

These are available in the `sc-data` repo:

https://github.com/suttacentral/sc-data

Pālifinder expects these to be in `/opt/sc/sc-flask/`, but again this can be specified at the command line.

## Using the `pali` command

All actions can be performed via a single command:

```cli
❯ pali
Usage: pali [OPTIONS] <COMMAND>

Commands:
  index        Create the index
  find         Search the index
  analyze      Display the tokens emitted by the text analyzer
  stem-table   Create the CSV stem file
  pali-chars   Display Pali characters expected by the tokenizer
  other-chars  Display all characters excluded by the tokenizer
  stop-words   List terms by frequency, descending
  help         Print this message or the help of the given subcommand(s)

Options:
      --dpd-db <DPD_DB>         Digital pali dictionary SQLite database file [default: data/dpd.db]
      --texts <TEXTS>           Directory containing Pali root texts [default: /opt/sc/sc-flask/sc-data/sc_bilara_data/root/pli/ms]
      --stem-file <STEM_FILE>   Location of file containing stems [default: data/stems.csv]
  -s, --stemmer <STEMMER>       Stemmer variant for text analyzer [default: snowball] [possible values: snowball, dictionary]
      --index-dir <INDEX_PATH>  Location of index [default: index]
  -h, --help                    Print help (see more with '--help')
```

## Creating the index

Using the Snowball stemmer is very straightforward.

```cli
pali index
```

The dictionary stemmer is a little more involved as the stem table needs to be created before indexing.

```cli
pali stem-table
pali --stemmer dictionary index
```

## Querying the index

Either stemmer can be used, but it's advisable to use the same stemmer that you indexed with. Any difference in the generated stems gives unexpected results.

Snowball:

```cli
pali find Evaṁ me sutaṁ
```

Dictionary:

```cli
pali --stemmer dictionary find Evaṁ me sutaṁ
```

## Development

### Building the Snowball stemmer

The Snowball code first needs to be compiled to rust. The compiler is available here:

https://github.com/snowballstem/snowball

Compiling the compiler was a single `make` command on my machine. YMMV.

Download the Snowball file:

https://github.com/khemarato/snowball/blob/pali/algorithms/pali.sbl

Compile to Rust:

```cli
./snowball -rust pali.sbl
```

This generates a `pali.rs` file. Copy it to the `src/snowball` directory.

The following edits are necessary:

Change these lines:

```rust
use snowball::SnowballEnv;
use snowball::Among;
```

to

```rust
use crate::snowball::snowball_env::SnowballEnv;
use crate::snowball::among::Among;
```

Clippy is set to "pedantic", so there might be a few more changes required if you want to issue a pull request.

### The stem table

When the `pali stem-table` command is run, it creates a vocabulary with all unique terms. Unlike other search engines tokenizer is aware of the Pāli alphabet. The `dpd.db` SQLite database is queried to obtain the stem for each term that is found. The Snowball stem is also computed and the result is a table in CSV format:

| term     | dpd_stem | snowball_stem |
| -------- | -------- | ------------- |
| niviṭṭho | niviṭṭh  | niviṭṭh       |

The `dpd_stem` column contains empty cells where there is no stem found in the database. Where values are missing, the dictionary stemmer uses the original term as the stem. 

`snowball_stem` always contains a stem.

### Testing the analyzers

You can check how the text analyzers handle arbitrary text:

```cli
❯ pali analyze "Pajāpatiṁ pajāpatito sañjānāti; "
pajāpat pajāpatit sañjān 
❯ pali --stemmer dictionary analyze "Pajāpatiṁ pajāpatito sañjānāti; "
pajāpatiṁ pajāpat sañjān 
```

This will work without the index being present.