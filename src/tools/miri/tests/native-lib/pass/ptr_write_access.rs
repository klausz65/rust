// Only works on Unix targets
//@ignore-target: windows wasm
//@only-on-host

use std::mem::MaybeUninit;

fn main() {
    test_modify_int();

    test_init_int();

    test_init_array();

    test_swap_ptr();

    test_dangling();
}

fn test_modify_int() {
    extern "C" {
        fn modify_int(ptr: *mut i32);
    }

    let mut x = 11;
    unsafe { modify_int(&mut x) };

    assert_eq!(x, 12);
}

fn test_init_int() {
    extern "C" {
        fn init_int(ptr: *mut i32);
    }

    let mut x = MaybeUninit::<i32>::uninit();
    let x = unsafe {
        init_int(x.as_mut_ptr());
        x.assume_init()
    };

    assert_eq!(x, 21);
}

fn test_init_array() {
    extern "C" {
        fn init_array(ptr: *mut i32, len: usize, value: i32);
    }

    const LEN: usize = 4;
    let init_value = 41;

    let mut array = MaybeUninit::<[i32; LEN]>::uninit();
    let array = unsafe {
        init_array(array.as_mut_ptr().cast::<i32>(), LEN, init_value);
        array.assume_init()
    };

    assert_eq!(array, [init_value; LEN]);
}

fn test_swap_ptr() {
    extern "C" {
        fn swap_ptr(pptr0: *mut *const i32, pptr1: *mut *const i32);
    }

    let x = 51;
    let mut ptr0 = &x;
    let mut ptr1 = std::ptr::null();
    unsafe { swap_ptr(&mut ptr0, &mut ptr1) };

    assert_eq!(unsafe { *ptr1 }, x);
}

fn test_init_static_inner() {
    extern "C" {
        fn init_static_inner(pptr: *const *mut MaybeUninit<i32>);
    }

    static mut INNER: MaybeUninit<i32> = MaybeUninit::uninit();
    static STATIC: *mut MaybeUninit<i32> = &raw mut INNER;
    unsafe { init_static_inner(&STATIC) }

    assert_eq!(unsafe { INNER.assume_init() }, 61);
}

fn test_dangling() {
    extern "C" {
        fn write_nullptr(pptr: *mut *const i32);
    }

    let x = 71;
    let mut ptr = &raw const x;
    drop(x);
    unsafe { write_nullptr(&mut ptr) };
    assert_eq!(ptr, std::ptr::null());
}

// TODO: Write tests for (forgetting to) expose: -initial allocation -recursively all allocations -unexposed pointers.