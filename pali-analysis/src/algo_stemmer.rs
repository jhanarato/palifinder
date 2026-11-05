use crate::snowball;
use tantivy::tokenizer::{Token, TokenFilter, TokenStream, Tokenizer};

#[derive(Clone)]
pub struct AlgorithmicStemmer;

impl TokenFilter for AlgorithmicStemmer {
    type Tokenizer<T: Tokenizer> = StemmerFilter<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> StemmerFilter<T> {
        StemmerFilter {
            inner: tokenizer,
        }
    }
}

#[derive(Clone)]
#[allow(unused)]
pub struct StemmerFilter<T> {
    inner: T,
}

impl<T: Tokenizer> Tokenizer for StemmerFilter<T> {
    type TokenStream<'a> = StemmerTokenStream<T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        StemmerTokenStream {
            tail: self.inner.token_stream(text),
            buffer: String::new(),
        }
    }
}

#[allow(unused)]
pub struct StemmerTokenStream<T> {
    tail: T,
    buffer: String,
}

impl<T: TokenStream> TokenStream for StemmerTokenStream<T> {
    fn advance(&mut self) -> bool {
        if !self.tail.advance() {
            return false;
        }
        let token = self.tail.token_mut();
        token.text = snowball::pali_stem(&token.text);
        true
    }

    fn token(&self) -> &Token {
        self.tail.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.tail.token_mut()
    }
}

#[cfg(test)]
mod tests {
    use crate::algo_stemmer::AlgorithmicStemmer;
    use crate::tests::assert_token;
    use tantivy::tokenizer::{TextAnalyzer, Token, WhitespaceTokenizer};

    fn token_stream_helper(text: &str) -> Vec<Token> {
        let stemmer = AlgorithmicStemmer;
        let mut token_stream = TextAnalyzer::builder(WhitespaceTokenizer::default())
            .filter(stemmer)
            .build();

        let mut token_stream = token_stream.token_stream(text);
        let mut tokens = vec![];
        let mut add_token = |token: &Token| {
            tokens.push(token.clone());
        };
        token_stream.process(&mut add_token);
        tokens
    }

    fn stem_all(words: &str) -> Vec<String> {
        words.split(' ').map(crate::snowball::pali_stem).collect()
    }

    fn assert_one_stem(words: &str, stem: &str) {
        for result in stem_all(words) {
            assert_eq!(result, stem, "in {words}");
        }
    }

    #[test]
    fn test_gati_stays_apart_from_gata() {
        assert_ne!(stem_all("sugati")[0], stem_all("sugato")[0]);
        assert_ne!(stem_all("duggati")[0], stem_all("duggato")[0]);
        assert_ne!(stem_all("gati")[0], stem_all("gato")[0]);
    }

    #[test]
    fn test_exceptions_hold_once_enclitics_are_off() {
        assert_one_stem("gati gatiñca gatiṁ", "gati");
        assert_one_stem("sugati sugatiñca", "sugati");
        assert_one_stem("duggati duggatiñca", "duggati");
        assert_one_stem("atthi atthīti", "atthi");
        assert_one_stem("natthi natthīti", "natthi");
        assert_one_stem("asīti", "asīti");
    }

    #[test]
    fn test_words_in_ant_that_are_not_participles_keep_it() {
        assert_one_stem("āmantā āmantesi", "āmant");
        assert_ne!(stem_all("pariyanto")[0], stem_all("pariyāyo")[0]);
        assert_ne!(stem_all("samantā")[0], stem_all("samo")[0]);
        assert_ne!(stem_all("āmantā")[0], stem_all("āmo")[0]);
    }

    #[test]
    fn test_noun_paradigms_share_a_stem() {
        assert_one_stem("dhammo dhammaṁ dhammassa dhammānaṁ dhammesu dhammena", "dhamm");
        assert_one_stem("bhikkhu bhikkhuno bhikkhūnaṁ bhikkhūhi", "bhikkh");
        assert_one_stem("paññā paññāya paññāyaṁ", "paññ");
        assert_one_stem("kāyo kāyena kāye", "kāy");
        assert_one_stem("nibbānaṁ nibbānassa nibbāne", "nibb");
        assert_one_stem("indriya indriyaṁ indriyassa indriyāni", "indriy");
        assert_one_stem("ariyo ariyā ariyaṁ ariyassa", "ariy");
        assert_one_stem("tathāgato tathāgate tathāgatassa", "tathāgat");
        assert_one_stem("rājā rājānaṁ rājāno", "rāj");
        assert_one_stem("brahmā brahmāno", "brahm");
        assert_one_stem("āpatti āpattiyo āpattiyā", "āpatt");
        assert_one_stem("manussa manusso manussā", "manuss");
        assert_one_stem("samatha samatho", "samath");
        assert_one_stem("ārāma ārāmo", "ārām");
        assert_one_stem("gahapati gahapatiṁ gahapatissa", "gahapat");
        assert_one_stem("duggati duggatiṁ duggatiyā", "duggati");
        assert_one_stem("sugati sugatiṁ", "sugati");
        assert_one_stem("gati gatiṁ gatīnaṁ", "gati");
        assert_one_stem("sabbe sabbesaṁ", "sabb");
        assert_one_stem("upādānaṁ upādānassa", "upād");
    }

    #[test]
    fn test_weak_and_consonant_stems_join_their_word() {
        assert_one_stem("bhagavā bhagavantaṁ bhagavato bhagavatā", "bhagav");
        assert_one_stem("āyasmā āyasmato", "āyasm");
        assert_one_stem("attā attano attanā", "att");
        assert_one_stem("pitā pitaraṁ", "pit");
    }

    #[test]
    fn test_verb_paradigms_share_a_stem() {
        assert_one_stem("gacchati gacchanti gacchāmi gacchissati gacchanto gacchamāno gacchamānaṁ", "gacch");
        assert_one_stem("bhāsati bhāsamāno", "bhās");
        assert_one_stem("pajānāti pajānāmi pajānātha pajānanti pajānaṁ", "pajān");
        assert_one_stem("parijānāti parijānātīti", "parijān");
        assert_one_stem("suṇāti suṇātu", "suṇ");
        assert_one_stem("vijjati vijjatī", "vijj");
    }

    #[test]
    fn test_enclitics_come_off() {
        assert_one_stem("navañca navampi", "nav");
        assert_one_stem("rūpañca rūpampi", "rūp");
        assert_one_stem("āpatti āpattīti āpattiñca", "āpatt");
        assert_one_stem("dhammeneva dhammopi", "dhamm");
        assert_one_stem("sabba sabbassāpi", "sabb");
        assert_one_stem("bhikkhuniyā bhikkhuniyāti", "bhikkhun");
    }

    #[test]
    fn test_both_niggahita_spellings_stem_alike() {
        assert_eq!(stem_all("dhammaṃ"), stem_all("dhammaṁ"));
    }

    #[test]
    fn test_words_that_only_look_inflected_keep_their_shape() {
        assert_eq!(stem_all("sappi papañca pañca"), vec!("sapp", "papañc", "pañc"));
        assert_one_stem("papañca papañce", "papañc");
        assert_one_stem("samāno samānaṁ samānassa", "samān");
        assert_eq!(stem_all("ukkhipi"), vec!("ukkhip"));
    }

    #[test]
    fn test_different_words_stay_apart() {
        let pairs = [
            ("āyasmā", "āyu"), ("ariyo", "ari"), ("atthi", "attho"), ("saddhiṁ", "saddhā"),
            ("iti", "ito"), ("dāni", "dānaṁ"), ("ahaṁ", "ahosi"), ("tatiyo", "tato"),
            ("pahīno", "pahoti"), ("manussa", "mano"),
        ];
        for (first, second) in pairs {
            assert_ne!(stem_all(first), stem_all(second), "{first} and {second}");
        }
    }

    #[test]
    fn test_empty_and_unusual_input_does_not_panic() {
        assert_eq!(stem_all(""), vec!(""));
        assert_eq!(stem_all("a"), vec!("a"));
        assert_eq!(stem_all("ṁ"), vec!("ṁ"));
        assert_eq!(stem_all("東京"), vec!("東京"));
    }

    #[test]
    fn test_algorithmic_stemmer() {
        let tokens = token_stream_helper("Tatra kho bhagavā");
        assert_token(&tokens[0], 0, "Tatr", 0, 5);
        assert_token(&tokens[1], 1, "kho", 6, 9);
        assert_token(&tokens[2], 2, "bhagav", 10, 18);
    }
}