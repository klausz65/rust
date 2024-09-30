// Only works on Unix targets
//@ignore-target: windows wasm
//@only-on-host

use std::mem::MaybeUninit;

fn main() {
    test_modify_int();

    test_init_int();

    test_init_array();

    test_swap_ptr();
}

fn test_modify_int() {
    extern "C" {
        fn modify_int(ptr: *mut i32);
    }

    let mut x = 1;
    unsafe { modify_int(&mut x) };

    assert_eq!(x, 3);
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

    assert_eq!(x, 29);
}

fn test_init_array() {
    extern "C" {
        fn init_array(ptr: *mut i32, len: usize, value: i32);
    }

    const LEN: usize = 4;
    let init_value = 5;

    let mut array = MaybeUninit::<[i32; LEN]>::uninit();
    let array = unsafe {
        init_array((*array.as_mut_ptr()).as_mut_ptr(), LEN, init_value);
        array.assume_init()
    };

    assert_eq!(array, [init_value; LEN]);
}

fn test_swap_ptr() {
    extern "C" {
        fn swap_ptr(x: *mut *const i32, y: *mut *const i32);
    }

    let x = 6;
    let [mut ptr0, mut ptr1] = [&x, std::ptr::null()];
    unsafe { swap_ptr(&mut ptr0, &mut ptr1); };

    assert_eq!(unsafe { *ptr1 }, x);
}