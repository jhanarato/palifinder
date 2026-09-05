//! Generated from pali.sbl by Snowball 3.1.1 - https://snowballstem.org/

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_mut)]
#![allow(unused_parens)]
#![allow(unused_variables)]
use crate::snowball::SnowballEnv;
use crate::snowball::Among;

#[derive(Clone)]
struct Context {
}

static A_0: &'static [Among<Context>; 25] = &[
    Among("a", -1, -1, None),
    Among("māna", 0, -1, None),
    Among("issa", 0, -1, None),
    Among("anta", 0, -1, None),
    Among("enta", 0, -1, None),
    Among("onta", 0, -1, None),
    Among("unta", 0, -1, None),
    Among("i", -1, -1, None),
    Among("esi", 7, -1, None),
    Among("āsi", 7, -1, None),
    Among("ati", 7, -1, None),
    Among("eti", 7, -1, None),
    Among("āti", 7, -1, None),
    Among("o", -1, -1, None),
    Among("ant", -1, -1, None),
    Among("u", -1, -1, None),
    Among("a\u{1E41}", -1, -1, None),
    Among("i\u{1E41}", -1, -1, None),
    Among("u\u{1E41}", -1, -1, None),
    Among("ā", -1, -1, None),
    Among("a\u{1E43}", -1, -1, None),
    Among("i\u{1E43}", -1, -1, None),
    Among("u\u{1E43}", -1, -1, None),
    Among("ī", -1, -1, None),
    Among("ū", -1, -1, None),
];

pub fn stem(env: &mut SnowballEnv) -> bool {
    let mut context = &mut Context {
    };
    env.limit_backward = env.cursor;
    env.cursor = env.limit;
    env.ket = env.cursor;
    if env.find_among_b(A_0, context) == 0 {
        return false;
    }
    env.bra = env.cursor;
    env.slice_del();
    env.cursor = env.limit_backward;
    return true
}
