use std::borrow::Cow;

mod among;
#[allow(unused)]
mod snowball_env;
mod pali;

#[must_use]
pub fn stem(input: &str) -> Cow<'_, str> {
    let mut env = snowball_env::SnowballEnv::create(input);
    pali::stem(&mut env);
    env.get_current()
}