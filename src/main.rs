use greenie::*;
#[greeny_main]
fn main() {
    println!("Hello!");
    let f = Fiber::new(|| println!("Hello from fiber!"));

    f.start().unwrap();
}

#[greeny_main]
fn foo() {}
