//! Regression test for issue #374, where previously rustc performed conditional jumps or moves that
//! incorrectly depended on uninitialized values.
//!
//! Issue: <https://github.com/rust-lang/rust/issues/374>.

//@ run-pass

#![allow(non_camel_case_types)]
#![allow(dead_code)]

enum sty {
    ty_nil,
}

struct RawT {
    struct_: sty,
    cname: Option<String>,
    hash: usize,
}

fn mk_raw_ty(st: sty, cname: Option<String>) -> RawT {
    return RawT { struct_: st, cname: cname, hash: 0 };
}

pub fn main() {
    mk_raw_ty(sty::ty_nil, None::<String>);
}
