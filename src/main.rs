use greenie::*;

async fn foo() {
    let f = Fiber::new(|| println!("Hello from fiber!"));
    f.await;
}
fn main() {
    println!("Hello!");

    futures::executor::block_on(foo());

    println!("Done!");
}
