use greenie::*;
use greenie_proc::greenify;

fn main() {
    let mut rt = Runtime::new(4, None);
    rt.init();
    rt.spawn(green_main);
    rt.run();
}
#[greenify]
fn __closure1() {
    println!("Thread #1 started");
    for i in 0..10 {
        println!("Thread #1: i = {}", i);
    }
}

#[greenify]
fn __closure2() {
    println!("Thread #2 started");
    for i in 0..10 {
        println!("Thread #2: i = {}", i);
    }
}

#[greenify]
fn green_main() {
    spawn_greenie(__closure1);
    spawn_greenie(__closure2);
}
