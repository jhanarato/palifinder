mod among;
#[allow(unused)]
#[allow(clippy::pedantic)]
#[allow(clippy::style)]
#[allow(clippy::complexity)]
#[allow(clippy::suspicious)]
mod snowball_env;
mod pali;

#[must_use]
pub fn pali_stem(input: &str) -> String {
    let mut env = snowball_env::SnowballEnv::create(input);
    pali::stem(&mut env);
    env.get_current().to_string()
}