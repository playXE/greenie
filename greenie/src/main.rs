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
            generator::generator_yield(1).unwrap();
            generator::generator_return("str").unwrap();
        },
        (),
    );

    match generator.resume().unwrap() {
        generator::GeneratorState::Yielded(val) => {
            assert!(val.is::<i32>());
        }
        _ => unreachable!(),
    }

    match generator.resume().unwrap() {
        generator::GeneratorState::Complete(val) => {
            assert!(val.is::<&'static str>());
        }
        _ => unreachable!(),
    }
}
