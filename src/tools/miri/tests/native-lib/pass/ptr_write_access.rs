// Only works on Unix targets
//@ignore-target: windows wasm
//@only-on-host

use std::mem::MaybeUninit;

fn main() {
    test_modify_int();

    test_init_int();

    test_init_array();

    test_swap_ptr();

    test_init_interior_mutable();

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
        fn init_array(ptr: *mut i32, len: usize);
    }

    const LEN: usize = 3;

    let mut array = MaybeUninit::<[i32; LEN]>::uninit();
    let array = unsafe {
        init_array(array.as_mut_ptr().cast::<i32>(), LEN);
        array.assume_init()
    };

    assert_eq!(array, [31; LEN]);
}

fn test_swap_ptr() {
    extern "C" {
        fn swap_ptr(pptr0: *mut *const i32, pptr1: *mut *const i32);
    }

    let x = 41;
    let mut ptr0 = &raw const x;
    let mut ptr1 = std::ptr::null();
    unsafe { swap_ptr(&mut ptr0, &mut ptr1) };

    assert_eq!(unsafe { *ptr1 }, x);
}

fn test_init_interior_mutable() {
    extern "C" {
        fn init_interior_mutable(pptr: *const UnsafeInterior);
    }

    #[repr(C)]
    struct UnsafeInterior {
        mut_ptr: *mut i32
    }
    unsafe impl Sync for UnsafeInterior {}
    
    let mut x = MaybeUninit::<i32>::uninit();
    let unsafe_interior = UnsafeInterior { mut_ptr: x.as_mut_ptr() };
    unsafe { init_interior_mutable(&unsafe_interior) };

    assert_eq!(unsafe { x.assume_init() }, 51);
}

fn test_dangling() {
    extern "C" {
        fn overwrite_ptr(pptr: *mut *const i32);
    }

    let x = vec![61];
    let mut ptr = x.as_ptr();
    drop(x);
    unsafe { overwrite_ptr(&mut ptr) };
    assert_eq!(ptr, std::ptr::null());
}

// TODO: Write tests for (forgetting to) expose: -initial allocation -recursively all allocations -unexposed pointers.
