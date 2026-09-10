use std::borrow::Cow;

mod among;
#[allow(unused)]
mod snowball_env;
mod pali;

#[must_use]
pub fn pali_stem(input: &str) -> String {
    let mut env = snowball_env::SnowballEnv::create(input);
    pali::stem(&mut env);
    match env.get_current() {
        Cow::Owned(stem) => stem,
        Cow::Borrowed(stem) => String::from(stem),
    }
}