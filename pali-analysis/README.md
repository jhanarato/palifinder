# pali-analysis

Pali text analysis for Tantivy search, used by palifinder:

- `tokenizer`: the Pali tokenizer and Unicode normalization (`nfc`).
- `snowball`: the Snowball stemming rules (`src/snowball/pali.sbl`), compiled to Rust with Snowball 3.2.0.
- `dict_stemmer` and `table`: the dictionary stemmer and the stem table it reads, in three modes: `Plain`, `Hybrid` (Snowball for words the dictionary lacks) and `Family` (a verb's forms grouped by root).
- `dpd` and `vocabulary`: building the stem table from the Digital Pali Dictionary's database and the texts' vocabulary.
- `analyzers`: the Tantivy analyzer for each stemmer.
- `fingerprint`: a hash of everything that shapes index terms, so that an index built with other rules or another stem table can be refused.

Every module here began as Ajahn Jhanarato's palifinder code. The changes since (the fuller Snowball rules, the word-based stem table, the hybrid and family modes, and the fingerprint) were written with Claude Code (Anthropic), directed by Dan Zigmond, and reviewed independently by OpenAI's Codex.

Text must be in Unicode's composed form (NFC) before it reaches the tokenizer or `Vocabulary`, both for indexing and for queries; `tokenizer::nfc` does this. The tokenizer does not normalize, so decomposed text (a macron typed as a combining mark) splits into fragments.

The stem table is built from DPD data; check DPD's licence before redistributing a built table.
