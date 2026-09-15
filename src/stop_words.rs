use crate::texts::Segment;
use counter::Counter;
use tantivy::tokenizer::{LowerCaser, TextAnalyzer, TokenStream};
use crate::tokenizer::PaliTokenizer;

#[allow(unused)]
pub fn most_frequent_words(segments: impl Iterator<Item = Segment>, number: usize) -> Vec<String> {
    let mut analyzer = TextAnalyzer::builder(PaliTokenizer::default())
        .filter(LowerCaser)
        .build();

    let mut tokens: Vec<String> = Vec::new();

    for segment in segments {
        let mut stream = analyzer.token_stream(segment.text.as_str());
        stream.process(&mut |token| {
            tokens.push(token.text.clone());
        });
    }

    let word_count: Counter<String> = tokens.iter().cloned().collect::<Counter<String>>();
    word_count.k_most_common_ordered(number).into_iter().map(|word_count| word_count.0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_most_frequent_words() {
        let segments = vec![
            Segment {
                uid: String::from("mn1:3.2"),
                text: String::from("pathaviṁ pathavito sañjānāti; "),
            },
            Segment {
                uid: String::from("mn1:3.3"),
                text: String::from(
                    "pathaviṁ pathavito saññatvā pathaviṁ maññati,\
             pathaviyā maññati, pathavito maññati, pathaviṁ meti maññati, pathaviṁ abhinandati. ",
                ),
            },
        ];
        assert_eq!(
            most_frequent_words(segments.into_iter(), 2),
            vec![String::from("pathaviṁ"), String::from("maññati")]
        );
    }
}
