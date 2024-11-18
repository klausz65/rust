// Testing that we don't fail abnormally after hitting the errors

use unresolved::*;
//~^ ERROR unresolved import `unresolved` [E0432]
//~| NOTE use of unresolved module or unlinked crate `unresolved`
//~| HELP if you wanted to use a crate named `unresolved`, use `cargo add unresolved` to add it to your `Cargo.toml`

fn main() {}
