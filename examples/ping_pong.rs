use greenie::common::Channel;

use greenie::{greeny_main, Fiber};
#[greeny_main]
fn main() {
    let chan_1 = Channel::<&'static str>::new(2);
    let chan_2 = Channel::<&'static str>::new(2);

    let fping = Fiber::new_capture(
        |chan_1, chan_2| {
            chan_1.send("ping");
            println!("{}", chan_2.recv().unwrap());
            chan_1.send("ping");
            println!("{}", chan_2.recv().unwrap());
            chan_1.send("ping");
            println!("{}", chan_2.recv().unwrap());
        },
        (chan_1.clone(), chan_2.clone()),
    );
    let fpong = Fiber::new_capture(
        |chan_1, chan_2| {
            chan_2.send("pong");
            println!("{}", chan_1.recv().unwrap());
            chan_2.send("pong");
            println!("{}", chan_1.recv().unwrap());
            chan_2.send("pong");
            println!("{}", chan_1.recv().unwrap());
        },
        (chan_1.clone(), chan_2.clone()),
    );

    fpong.start().unwrap();
    fping.start().unwrap();

    fpong.join().unwrap();
    fping.join().unwrap();
    println!("Finished");
}
