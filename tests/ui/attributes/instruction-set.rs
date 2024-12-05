#![feature(stmt_expr_attributes)]

#[cfg(target_arch = "arm")] // OK
#[instruction_set(arm::a32)]
fn valid_a() {}

#[cfg(target_arch = "arm")] // OK
#[instruction_set(arm::t32)]
fn valid_b() {}

#[cfg(target_arch = "arm")] // OK
struct MyStruct;

#[cfg(target_arch = "arm")]
impl MyStruct {
    #[instruction_set(arm::a32)] // OK
    fn inherent_method(&self) {}
}

trait MyTrait {
    #[cfg(target_arch = "arm")]
    #[instruction_set(arm::a32)] // OK
    fn trait_method() {
        println!("Trait method default implementation");
    }
}

struct A;
impl MyTrait for A {
    #[cfg(target_arch = "arm")]
    #[instruction_set(arm::t32)] // OK
    fn trait_method() {
        println!("Trait impl method");
    }
}

#[instruction_set(asdfasdf)] //~ ERROR The `[instruction_set]` attribute is only allowed on functions
type InvalidA = ();

#[instruction_set(asdfasdf)] //~ ERROR The `[instruction_set]` attribute is only allowed on functions
mod InvalidB {}

#[instruction_set(asdfasdf)] //~ ERROR The `[instruction_set]` attribute is only allowed on functions
struct InvalidC;

#[instruction_set(asdfasdf)] //~ ERROR `[instruction_set]` attribute argument should be valid
fn invalid_d() {}

fn main() {}
