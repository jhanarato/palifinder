use std::borrow::Cow;

mod among;
mod snowball_env;

pub mod algorithms;

#[derive(Clone)]
pub struct Stemmer {
    stemmer: fn(&mut snowball_env::SnowballEnv) -> bool,
}

impl Stemmer {
    #[must_use]
    pub fn create() -> Self {
        Stemmer { stemmer: algorithms::pali::stem }
    }

    #[must_use]
    pub fn stem<'a>(&self, input: &'a str) -> Cow<'a, str> {
        let mut env = snowball_env::SnowballEnv::create(input);
        (self.stemmer)(&mut env);
        env.get_current()
    }
}