//! Pali text analysis for search: the tokenizer, the Snowball and dictionary
//! stemmers, and the stem table they read, built from the Digital Pali Dictionary.
//!
//! Callers must pass text in Unicode's composed form (NFC), as palifinder does with
//! [`tokenizer::nfc`] for texts and queries alike. The tokenizer does not normalize:
//! decomposed text, such as a macron typed as a combining mark, splits into
//! fragments, so `nibba\u{304}nassa` stems as two terms instead of one.

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

const ANALYZER_SOURCES: [&str; 7] = [
    include_str!("snowball/pali.rs"),
    include_str!("snowball/snowball_env.rs"),
    include_str!("snowball.rs"),
    include_str!("tokenizer.rs"),
    include_str!("analyzers.rs"),
    include_str!("algo_stemmer.rs"),
    include_str!("dict_stemmer.rs"),
];

/// 64-bit FNV-1a. `DefaultHasher` may change between Rust releases, and an old
/// index must not look out of date merely because the toolchain moved on.
fn fnv1a(hash: u64, bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(hash, |hash, &byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
}

/// Changes when anything that shapes the index terms does, so that an index built
/// with older rules, analyzers or stem table is not searched with newer ones.
#[must_use]
pub fn fingerprint(stem_table: Option<&[u8]>) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for source in ANALYZER_SOURCES {
        hash = fnv1a(hash, source.as_bytes());
    }
    stem_table.map_or(hash, |table| fnv1a(hash, table))
}

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn test_fnv1a_is_the_published_function() {
        assert_eq!(fnv1a(0xcbf2_9ce4_8422_2325, b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn test_the_fingerprint_depends_on_the_table() {
        assert_ne!(fingerprint(Some(b"a")), fingerprint(Some(b"b")));
        assert_ne!(fingerprint(None), fingerprint(Some(b"a")));
    }
}
