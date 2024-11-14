//@ compile-flags: -O -Z merge-functions=disabled

#![crate_type = "lib"]

use std::rc::Rc;
use std::sync::Arc;

// CHECK-LABEL: @deref_rc_sized(
// CHECK-NOT: getelementptr
// CHECK: ret
#[no_mangle]
pub fn deref_rc_sized(rc: &Rc<u32>) -> &u32 {
    &rc
}

// CHECK-LABEL: @deref_rc_unsized(
// CHECK-COUNT-1: getelementptr
// CHECK: ret
#[no_mangle]
pub fn deref_rc_unsized(rc: &Rc<str>) -> &str {
    &rc
}

// CHECK-LABEL: @deref_arc_sized(
// CHECK-NOT: getelementptr
// CHECK: ret
#[no_mangle]
pub fn deref_arc_sized(arc: &Arc<u32>) -> &u32 {
    &arc
}

// CHECK-LABEL: @deref_arc_unsized(
// CHECK-COUNT-1: getelementptr
// CHECK: ret
#[no_mangle]
pub fn deref_arc_unsized(arc: &Arc<str>) -> &str {
    &arc
}

// CHECK-LABEL: @rc_slice_to_ref_slice_sized(
// CHECK-NOT: getelementptr
// CHECK: tail call void @llvm.memcpy
// CHECK-COUNT-1: getelementptr
// CHECK: ret
#[no_mangle]
pub fn rc_slice_to_ref_slice_sized(s: &[Rc<u32>]) -> Box<[&u32]> {
    s.iter().map(|x| &**x).collect()
}

// This test doesn’t work yet.
//
// COM: CHECK-LABEL: @rc_slice_to_ref_slice_unsized(
// COM: CHECK-NOT: getelementptr
// COM: CHECK: tail call void @llvm.memcpy
// COM: CHECK-NOT: getelementptr
// COM: CHECK: ret
// #[no_mangle]
// pub fn rc_slice_to_ref_slice_unsized(s: &[Rc<str>]) -> Box<[&str]> {
//     s.iter().map(|x| &**x).collect()
// }

// CHECK-LABEL: @arc_slice_to_ref_slice_sized(
// CHECK-NOT: getelementptr
// CHECK: tail call void @llvm.memcpy
// CHECK-COUNT-1: getelementptr
// CHECK: ret
#[no_mangle]
pub fn arc_slice_to_ref_slice_sized(s: &[Arc<u32>]) -> Box<[&u32]> {
    s.iter().map(|x| &**x).collect()
}

// This test doesn’t work yet.
//
// COM: CHECK-LABEL: @arc_slice_to_ref_slice_unsized(
// COM: CHECK-NOT: getelementptr
// COM: CHECK: tail call void @llvm.memcpy
// COM: CHECK-NOT: getelementptr
// COM: CHECK: ret
// #[no_mangle]
// pub fn arc_slice_to_ref_slice_unsized(s: &[Arc<str>]) -> Box<[&str]> {
//     s.iter().map(|x| &**x).collect()
// }
