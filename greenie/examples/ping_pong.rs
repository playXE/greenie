use greenie::channel::*;

use greenie::{greeny_main, Fiber};
#[greeny_main]
fn main() {
    let chan_1 = Channel::<&'static str>::new(2);
    let chan_2 = Channel::<&'static str>::new(2);

    let fping = Fiber::new_capture(
        |chan_1, chan_2| {
            chan_1.push("ping");
            println!("{}", chan_2.pop().unwrap());
            chan_1.push("ping");
            println!("{}", chan_2.pop().unwrap());
            chan_1.push("ping");
            println!("{}", chan_2.pop().unwrap());
        },
        (chan_1.clone(), chan_2.clone()),
    );
    let fpong = Fiber::new_capture(
        |chan_1, chan_2| {
            chan_2.push("pong");
            println!("{}", chan_1.pop().unwrap());
            chan_2.push("pong");
            println!("{}", chan_1.pop().unwrap());
            chan_2.push("pong");
            println!("{}", chan_1.pop().unwrap());
        },
        (chan_1.clone(), chan_2.clone()),
    );

    fpong.start().unwrap();
    fping.start().unwrap();
}