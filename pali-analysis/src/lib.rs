//! Pali text analysis for search: the tokenizer, the Snowball and dictionary
//! stemmers, and the stem table they read, built from the Digital Pali Dictionary.

mod algo_stemmer;
pub mod analyzers;
mod dict_stemmer;
pub mod dpd;
pub mod snowball;
pub mod table;
#[cfg(test)]
mod tests;
pub mod tokenizer;
pub mod vocabulary;
