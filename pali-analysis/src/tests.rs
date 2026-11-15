use tantivy::tokenizer::Token;

/// Helper function for testing token output. Copied from the `tantivy::tokenizer` tests.
#[allow(clippy::missing_panics_doc)]
pub fn assert_token(token: &Token, position: usize, text: &str, from: usize, to: usize) {
    assert_eq!(
        token.position, position,
        "expected position {position} but {token:?}"
    );
    assert_eq!(token.text, text, "expected text {text} but {token:?}");
    assert_eq!(
        token.offset_from, from,
        "expected offset_from {from} but {token:?}"
    );
    assert_eq!(token.offset_to, to, "expected offset_to {to} but {token:?}");
}