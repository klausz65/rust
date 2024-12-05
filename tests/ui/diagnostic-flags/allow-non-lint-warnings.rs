// ignore-tidy-linelength
//! Check that `-A warnings` cli flag applies to *all* warnings, including feature gate warnings.
//!
//! This test tries to exercise that by checking that the "empty trait list for derive" warning for
//! `#[derive()]` is permitted by `-A warnings`, which is a non-lint warning.
//!
//! # Relevant context
//!
//! - Original impl PR: <https://github.com/rust-lang/rust/pull/21248>.
//! - RFC 507 "Release channels":
//!   <https://github.com/rust-lang/rfcs/blob/c017755b9bfa0421570d92ba38082302e0f3ad4f/text/0507-release-channels.md>.

//@ compile-flags: -Awarnings
//@ check-pass

#[derive()]
#[derive(Copy, Clone)]
pub struct Foo;

pub fn main() {}
