// Testing that we don't fail abnormally after hitting the errors

use unresolved::*;
//~^ ERROR unresolved import `unresolved` [E0432]
//~| NOTE use of unresolved module or unlinked crate `unresolved`
//~| HELP consider importing the `unresolved` crate

fn main() {}
