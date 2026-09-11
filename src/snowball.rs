mod among;
#[allow(unused)]
mod snowball_env;
mod pali;

#[must_use]
pub fn pali_stem(input: &str) -> String {
    let mut env = snowball_env::SnowballEnv::create(input);
    pali::stem(&mut env);
    env.get_current().to_string()
}