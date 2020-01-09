use generator::*;
use greenie::*;

fn main() {
    let x = 42;
    let y = "Hello!";
    let generator = Generator::spawn(
        |x, y| {
            generator_yield(x).unwrap();
            generator_yield(y).unwrap();

            return "Complete";
        },
        (x, y),
    );

    let result = iterate_generator! {
        for (x in generator) {

            if x.is::<i32>() {
                println!("{}",x.downcast::<i32>().unwrap());
            } else if x.is::<&'static str> () {
                println!("{}",x.downcast::<&'static str>().unwrap());
            }
        }
    };

    println!("{}", result.downcast::<&'static str>().unwrap());
}
