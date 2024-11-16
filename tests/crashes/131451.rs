//@ known-bug: #131451
//@ needs-rustc-debug-assertions
//@ compile-flags: -Zmir-opt-level=5 -Zvalidate-mir

fn check_multiple_lints_3(terminate: bool) {
    while true {}

    while !terminate {}
}
