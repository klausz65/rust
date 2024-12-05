//! Check that we don't get compile errors on unreachable code after the `{ ret 3; }` artificial
//! block below. This test is run-pass to also exercise the codegen, but it might be possible to
//! reduce to build-pass or even check-pass.
//!
//! This test was introduced as part of commit `a833f152baa17460e8414355e832d30d5161f8e8` which
//! removes an "artificial block". See also commit `8d381823e2aa1524eabeb3219d7dc1d5007e6096` for
//! more elaboration, produced below (this is outdated for *today*'s rustc as of Dec 05, 2024, but
//! is helpful to understand the original intention):
//!
//! > Return a fresh, unreachable context after ret, break, and cont
//! >
//! > This ensures we don't get compile errors on unreachable code (see
//! > test/run-pass/artificial-block.rs for an example of sane code that wasn't compiling). In the
//! > future, we might want to warn about non-trivial code appearing in an unreachable context,
//! > and/or avoid generating unreachable code altogether (though I'm sure LLVM will weed it out as
//! > well).
//!
//! Since then, `ret` -> `return`, `int` -> `isize`, `assert` became a macro.

//@ run-pass

fn f() -> isize {
    {
        return 3;
    }
}

fn main() {
    assert_eq!(f(), 3);
}
