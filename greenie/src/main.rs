use greenie::*;
use scheduler::*;
fn main() {
    RUNTIME.with(|rt| {
        rt.get().spawn(green_main, ());
        rt.get().run();
    })
}

fn green_main() {
    let generator = generator::Generator::spawn(
        || {
            generator_yield(1).unwrap();
            generator_return(2).unwrap();
        },
        (),
    );
}
