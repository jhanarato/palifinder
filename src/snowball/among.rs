/// Copied from here: <https://github.com/snowballstem/snowball/blob/main/rust/src/snowball/among.rs>
use crate::snowball::snowball_env::SnowballEnv;

#[allow(clippy::type_complexity)]
pub struct Among<T: 'static>(pub &'static str,
                             pub i32,
                             pub i32,
                             pub Option<&'static (dyn Fn(&mut SnowballEnv, &mut T) -> bool + Sync)>);
