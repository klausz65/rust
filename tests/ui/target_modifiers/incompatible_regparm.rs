//@ aux-crate:wrong_regparm=wrong_regparm.rs
//@ compile-flags: --target i686-unknown-linux-gnu -Zregparm=1 -Cpanic=abort
//@ needs-llvm-components: x86
//@ revisions:error_generated allow_regparm_mismatch allow_any_mismatch allow_attr

//@[allow_regparm_mismatch] compile-flags: -Cunsafe-allow-abi-mismatch=regparm
//@[allow_any_mismatch] compile-flags: -Cunsafe-allow-abi-mismatch
//@[allow_regparm_mismatch] build-pass
//@[allow_any_mismatch] build-pass
//@[allow_attr] build-pass

#![crate_type = "lib"]
//[error_generated]~^ ERROR 12:1: 12:1: mixing `-Zregparm` will cause an ABI mismatch [incompatible_target_modifiers]
#![no_core]
#![feature(no_core, lang_items, repr_simd)]

#![cfg_attr(allow_attr, allow(incompatible_target_modifiers))]

fn foo() {
    wrong_regparm::somefun();
}
