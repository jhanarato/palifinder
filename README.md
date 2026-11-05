# Pālifinder

Pālifinder is a command line search engine for the Pāli Canon, written in Rust and using the [Tantivy](https://github.com/quickwit-oss/tantivy) crate.

<img width="1859" height="572" alt="palifinder" src="https://github.com/user-attachments/assets/e87ab2b2-c1e2-4016-99dc-70893cef0e4a" />

Tantivy is comparable to [Apache Lucene](https://lucene.apache.org/), the indexing engine used by ElasticSearch. 

For the most part Pālifinder should be considered a prototype, though it is certainly usable and hopefully useful. It includes:

- A tokenizer that recognizes Pāli, as encoded in SuttaCentral's segmented Pāli texts.
- Four stemmers: dictionary lookup; Snowball compiled to pure Rust; a hybrid that falls back to Snowball for terms the dictionary lacks; and a family stemmer that also groups the verbal forms of each root.
- A stopword filter.
- Indexing of SuttaCentral's segmented Pāli texts.
- Search queries with keywords-in-context display.  

As a prototype, it has limitations:

- The Pāli language only, not the translated texts.
- Command line only. No REST API.
- The dictionary stemmer has a word for about 73% of the vocabulary. Most of the rest are compounds and sandhi forms that DPD holds only in its deconstructor; the hybrid stemmer sends these to Snowball.
- Numerals and a few pronoun forms DPD lists under their own headwords stay apart: tīṇi does not find tayo, and maṁ does not find mayhaṁ.
- Only the family stemmer joins suppletive verb forms, so that gacchati finds gata. Negated forms stay out of the family, so appaṭivirata does not find paṭivirata, and causatives have families of their own.
- Default Tantivy query parser. [There's a lot more in the Tantivy toolchest](https://docs.rs/tantivy/latest/tantivy/query/index.html).

## Why several stemmers?

Our algorithmic stemmer began as Ven. Khemarato's Snowball program and is compiled to Rust. Its source is `pali-analysis/src/snowball/pali.sbl`. Pālifinder allows us to test the algorithm and make it production ready.

The dictionary stemmer stems (pun intended) from a conversation with the [Digital Pāli Dictionary](https://dpdict.net/) folk. Given that the hard work has already been done to match a stem to every word in the dictionary, we should be able to extract the stems for each word, store it as a map in memory and use the map as a lookup table for the stemmer. DPD's stems do not identify words: unrelated words share them, and irregular forms have none. So the table maps each term to its DPD word instead, with the root telling homonyms apart. An ambiguous term goes to the reading the texts best support, estimated across the whole table, and a term spelled like a headword leans to that headword. `PREFERRED_ROOTS` in `pali-analysis/src/dpd.rs` settles a few common words no count can, and `stem-table` fails if DPD no longer has one of those readings, as after renumbering a root. A term DPD knows only as a word plus enclitics goes with that word. The hybrid and family stemmers use Snowball for the rest: a compound or an unseen query form takes the commonest word with its Snowball stem, unless terms DPD does not know outnumber that word. The table also holds DPD's headwords, so that a citation form the texts happen not to contain, such as manussa, still finds its word.

The other issue I wanted to address was how much memory the map would use. As it turns out we're looking at about 11MB of data on disk, so a trivial amount of memory. The speed of the lookup is pretty similar, though the Snowball implementation may become slower as it gets more complex.

Really there's no reason not to persue both.

## Installation & setup

As this is a prototype, there are no releases. To build and install:

- Install the Rust toolchain for your platform: https://rustup.rs/
- In the source directory run `cargo install`

### Dictionary stemmer

By default, Pālifinder uses the included Snowball stemmer. If you want to use or test the dictionary stemmer, you'll need the SQLite database of the Digital Pāli Dictionary to compile a stem table. 

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
  stem-words   Print each line of standard input followed by a tab and its analyzed tokens
  pali-chars   Display Pali characters expected by the tokenizer
  other-chars  Display all characters excluded by the tokenizer
  stop-words   List terms by frequency, descending
  help         Print this message or the help of the given subcommand(s)

Options:
      --dpd-db <DPD_DB>         Digital pali dictionary SQLite database file [default: data/dpd.db]
      --texts <TEXTS>           Directory containing Pali root texts [default: /opt/sc/sc-flask/sc-data/sc_bilara_data/root/pli/ms]
      --stem-file <STEM_FILE>   Location of file containing stems [default: data/stems.csv]
  -s, --stemmer <STEMMER>       Stemmer variant for text analyzer [possible values: snowball, dictionary, hybrid, family]
      --index-dir <INDEX_PATH>  Location of index [default: index]
  -h, --help                    Print help (see more with '--help')
```

## Creating the index

Using the Snowball stemmer is very straightforward.

```cli
pali index
```

The other stemmers are a little more involved as the stem table needs to be created before indexing.

```cli
pali stem-table
pali --stemmer hybrid index
```

## Querying the index

The index records its stemmer, and `pali find` uses it. It refuses a different stemmer, and refuses to search if the rules or stem table have changed since indexing, because queries stemmed differently from the index quietly find too little.

```cli
pali find Evaṁ me sutaṁ
```

## Development

### Building the Snowball stemmer

The Snowball code first needs to be compiled to rust. The compiler is available here:

https://github.com/snowballstem/snowball

Compiling the compiler was a single `make` command on my machine. YMMV.

Compile the source to Rust:

```cli
./snowball pali-analysis/src/snowball/pali.sbl -rust -o pali
```

This generates a `pali.rs` file. Copy it to the `pali-analysis/src/snowball` directory.

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

`pali-analysis/src/snowball/snowball_env.rs` comes from the same Snowball release as the compiler, currently 3.2.0. Copy it across when you update the compiler, with its `use snowball::Among;` changed as above: generated code does not build against an older runtime.

### The stem table

When the `pali stem-table` command is run, it creates a vocabulary with all unique terms, tokenizing on the Pāli alphabet. The `dpd.db` SQLite database is queried to obtain the stem for each term that is found. The Snowball stem is also computed and the result is a table in CSV format:

| term     | dpd_stem       | snowball_stem | family               | fallback       |
| -------- | -------------- | ------------- | -------------------- | -------------- |
| niviṭṭho | niviṭṭha | niviṭṭh       | family:ni √vis\|√vis 1\| | niviṭṭha |

`dpd_stem` holds the term's DPD word, and is empty when DPD has none. In this case the dictionary stemmer will emit the term unchanged. `fallback` is the commonest DPD word with the same Snowball stem.

`snowball_stem` always contains a stem. The table is sorted by term, so a rebuild from the same data is identical.

### Testing the analyzers

You can analyze arbitrary text without indexing:

```cli
❯ pali analyze "Pajāpatiṁ pajāpatito sañjānāti; "
pajāpat pajāpatit sañjān 
❯ pali --stemmer hybrid analyze "Pajāpatiṁ pajāpatito sañjānāti; "
pajāpati pajāpati sañjānāti 
```