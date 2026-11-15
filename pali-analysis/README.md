# pali-analysis

The Pali tokenizer and stemmers palifinder indexes with, as a crate of their own so
that another search backend can depend on them:

- `tokenizer`: the tokenizer for SuttaCentral's Pali texts, and Unicode normalization (`nfc`).
- `snowball`: the Snowball stemmer, compiled to Rust from `src/snowball/pali.sbl`.
- `table` and `dpd`: the stem table, built from the Digital Pali Dictionary.
- `analyzers`: Tantivy text analyzers combining these.
- `fingerprint`: a hash of everything that shapes index terms, so that an index built with other rules or another stem table can be refused.

Text must be in Unicode's composed form (NFC) before it reaches the tokenizer or `Vocabulary`, both for indexing and for queries; `tokenizer::nfc` does this.
