use crate::app::test_0::RandomTest0;

mod app;

fn main() {
    println!("Start");

    // RandomTest0::run_0(1000, 2, 3);
    RandomTest0::run_0(1000, 40, 5);

    println!("End");
}
