# pali-analysis

The Pali tokenizer and stemmers palifinder indexes with, as a crate of their own so
that another search backend can depend on them:

- `tokenizer`: the tokenizer for SuttaCentral's Pali texts.
- `snowball`: the Snowball stemmer, compiled to Rust from `src/snowball/pali.sbl`.
- `table` and `dpd`: the stem table, built from the Digital Pali Dictionary.
- `analyzers`: Tantivy text analyzers combining these.
