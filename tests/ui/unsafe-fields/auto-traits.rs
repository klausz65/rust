//@ compile-flags: --crate-type=lib

#![feature(auto_traits)]
#![feature(unsafe_fields)]
#![allow(dead_code, incomplete_features, unconditional_recursion)]

enum UnsafeEnum {
    Safe(u8),
    Unsafe { unsafe field: u8 },
}

auto trait SafeAuto {}

fn impl_safe_auto(_: impl SafeAuto) {
    impl_safe_auto(UnsafeEnum::Safe(42))
}

unsafe auto trait UnsafeAuto {}

fn impl_unsafe_auto(_: impl UnsafeAuto) {
    impl_unsafe_auto(UnsafeEnum::Safe(42))
    //~^ ERROR the trait bound `UnsafeEnum: UnsafeAuto` is not satisfied
}
