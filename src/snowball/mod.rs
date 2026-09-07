use std::borrow::Cow;

mod among;
mod snowball_env;

pub mod algorithms;

#[must_use]
pub fn stem(input: &str) -> Cow<'_, str> {
    let mut env = snowball_env::SnowballEnv::create(input);
    algorithms::pali::stem(&mut env);
    env.get_current()
}
