// Only works on Unix targets
//@ignore-target: windows wasm
//@only-on-host

#![feature(box_as_ptr)]

use std::mem::MaybeUninit;
use std::ptr::null;

fn main() {
    test_increment_int();

    test_init_int();

    test_init_array();

    test_init_static_inner();

    test_expose_int();

    test_swap_ptr();

    test_swap_nested_ptr();

    test_swap_tuple();

    test_overwrite_dangling();

    test_expose_triple();
}

// Test function that modifies an int.
fn test_increment_int() {
    extern "C" {
        fn increment_int(ptr: *mut i32);
    }

    let mut x = 11;

    unsafe { increment_int(&mut x) };
    assert_eq!(x, 12);
}

// Test function that initializes an int.
fn test_init_int() {
    extern "C" {
        fn init_int(ptr: *mut i32, val: i32);
    }

    let mut x = MaybeUninit::<i32>::uninit();
    let val = 21;

    let x = unsafe {
        init_int(x.as_mut_ptr(), val);
        x.assume_init()
    };
    assert_eq!(x, val);
}

// Test function that initializes an array.
fn test_init_array() {
    extern "C" {
        fn init_array(ptr: *mut i32, len: usize, val: i32);
    }

    const LEN: usize = 3;
    let mut array = MaybeUninit::<[i32; LEN]>::uninit();
    let val = 31;
    
    let array = unsafe {
        init_array(array.as_mut_ptr().cast::<i32>(), LEN, val);
        array.assume_init()
    };
    assert_eq!(array, [val; LEN]);
}

// Test function that initializes an int pointed to by an immutable static.
fn test_init_static_inner() {
    #[repr(C)]
    struct SyncPtr {
        ptr: *mut i32
    }
    unsafe impl Sync for SyncPtr {}

    extern "C" {
        fn init_static_inner(s_ptr: *const SyncPtr, val: i32);
    }

    static mut INNER: MaybeUninit<i32> = MaybeUninit::uninit();
    #[allow(static_mut_refs)]
    static STATIC: SyncPtr = SyncPtr { ptr: unsafe { INNER.as_mut_ptr() } };
    let val = 41;

    let inner = unsafe {
        init_static_inner(&STATIC, val);
        INNER.assume_init()
    };
    assert_eq!(inner, val);
}

// Test function that writes a pointer and exposes the alloc of its int argument.
fn test_expose_int() {
    extern "C" {
        fn expose_int(int_ptr: *const i32, pptr: *mut *const i32);
    }

    let x = 51;
    let mut ptr = std::ptr::null();

    unsafe { expose_int(&x, &mut ptr) };
    assert_eq!(unsafe { *ptr }, x);
}

// Test function that swaps two pointers and exposes the alloc of an int.
fn test_swap_ptr() {
    extern "C" {
        fn swap_ptr(pptr0: *mut *const i32, pptr1: *mut *const i32);
    }

    let x = 61;
    let (mut ptr0, mut ptr1) = (&raw const x, null());

    unsafe { swap_ptr(&mut ptr0, &mut ptr1) };
    assert_eq!(unsafe { *ptr1 }, x);
}

// Test function that swaps two nested pointers and exposes the alloc of an int.
fn test_swap_nested_ptr() {
    extern "C" {
        fn swap_nested_ptr(ppptr0: *mut *mut *const i32, ppptr1: *mut *mut *const i32);
    }

    let x = 71;
    let (mut ptr0, mut ptr1) = (&raw const x, null());
    let (mut pptr0, mut pptr1) = (&raw mut ptr0, &raw mut ptr1);

    unsafe { swap_nested_ptr(&mut pptr0, &mut pptr1) }
    assert_eq!(unsafe { *ptr1 }, x);
}

// Test function that swaps two pointers in a struct and exposes the alloc of an int.
fn test_swap_tuple() {
    #[repr(C)]
    struct Tuple {
        ptr0: *const i32,
        ptr1: *const i32,
    }

    extern "C" {
        fn swap_tuple(t_ptr: *mut Tuple);
    }

    let x = 81;
    let mut tuple = Tuple { ptr0: &raw const x, ptr1: null() };

    unsafe { swap_tuple(&mut tuple) }
    assert_eq!(unsafe { *tuple.ptr1 }, x);
}

// Test function that interacts with a dangling pointer.
fn test_overwrite_dangling() {
    extern "C" {
        fn overwrite_ptr(pptr: *mut *const i32);
    }

    let b = Box::new(91);
    let mut ptr = Box::as_ptr(&b);
    drop(b);
    unsafe { overwrite_ptr(&mut ptr) };

    assert_eq!(ptr, null());
}

// Test function that interacts with a struct storing a dangling pointer.
fn test_expose_triple() {
    #[repr(C)]
    struct Triple {
        ptr0: *const i32,
        ptr1: *const i32,
        ptr2: *const i32,
    }

    extern "C" {
        fn expose_triple(t_ptr: *const Triple);
    }

    let x = 101;
    let y = 111;
    let b = Box::new(121);
    let ptr = Box::as_ptr(&b);
    drop(b);
    let triple = Triple { ptr0: &raw const x, ptr1: ptr, ptr2: &raw const y };

    unsafe { expose_triple(&triple) }
    assert_eq!(unsafe { *triple.ptr2 }, y);
}
