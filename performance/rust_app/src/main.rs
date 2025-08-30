use std::env;
use std::collections::HashMap;
use x_overlay::core::overlay_rule::OverlayRule;
use crate::test::test_0_checkerboard::CheckerboardTest;
use crate::test::test_1_not_overlap::NotOverlapTest;
use crate::test::test_2_lines_net::LinesNetTest;
use crate::test::test_3_wind_mill::WindMillTest;
use crate::test::test_4_windows::WindowsTest;
use crate::test::test_5_nested_squares::CrossTest;

mod test;

fn main() {
    let args = env::args();
    let mut args_iter = args.peekable();
    let mut args_map = HashMap::new();

    while let Some(arg) = args_iter.next() {
        if arg.starts_with("--") {
            let key = arg.trim_start_matches("--").to_owned();
            // If the next argument is also a key, store a boolean flag; otherwise, store the value.
            let value = if args_iter.peek().map_or(false, |a| a.starts_with("--")) {
                "true".to_string()
            } else {
                args_iter.next().unwrap()
            };
            args_map.insert(key, value);
        }
    }

    #[cfg(debug_assertions)]
    {
        if args_map.is_empty() {
            args_map.insert("multithreading".to_string(), "false".to_string());
            args_map.insert("complex".to_string(), "false".to_string());
            args_map.insert("test".to_string(), 0.to_string());
            let count = 32;
            args_map.insert("count".to_string(), count.to_string());
        }
    }

    let test_key = args_map.get("test").expect("Test number is not set");
    let multithreading_key = args_map.get("multithreading").expect("Multithreading is not set");
    let complex_key = args_map.get("complex").expect("Complex is not set");


    let test: usize = test_key.parse().expect("Unable to parse test as an integer");
    let multithreading: bool = multithreading_key.parse().expect("Unable to parse multithreading as an boolean");
    let complex: bool = complex_key.parse().expect("Unable to parse complex as an boolean");


    if complex {
        match test {
            0 => {
                run_test_0(multithreading);
            }
            1 => {
                run_test_1(multithreading);
            }
            2 => {
                run_test_2(multithreading);
            }
            3 => {
                run_test_3(multithreading);
            }
            4 => {
                run_test_4(multithreading);
            }
            5 => {
                run_test_5(multithreading);
            }
            _ => {
                println!("Test is not found");
            }
        }
    } else {
        let count_key = args_map.get("count").expect("Count is not set");
        let count: usize = count_key.parse().expect("Unable to parse count as an integer");
        match test {
            0 => {
                CheckerboardTest::run(count, OverlayRule::Xor, 1.0, multithreading);
            }
            1 => {
                NotOverlapTest::run(count, OverlayRule::Union, 1.0, multithreading);
            }
            2 => {
                LinesNetTest::run(count, OverlayRule::Intersect, 1.0, multithreading);
            }
            3 => {
                WindMillTest::run(count, OverlayRule::Intersect, 1.0, multithreading);
            }
            4 => {
                WindowsTest::run(count, OverlayRule::Difference, 1.0, multithreading);
            }
            5 => {
                CrossTest::run(count, OverlayRule::Xor, 1.0, multithreading);
            }
            _ => {
                println!("Test is not found");
            }
        }
    }
}

fn run_test_0(multithreading: bool) {
    println!("run Checkerboard test");
    for i in 1..12 {
        let n = 1 << i;
        CheckerboardTest::run(n, OverlayRule::Xor, 1000.0, multithreading);
    }
}

fn run_test_1(multithreading: bool) {
    println!("run NotOverlap test");
    for i in 1..12 {
        let n = 1 << i;
        NotOverlapTest::run(n, OverlayRule::Xor, 1000.0, multithreading);
    }
}

fn run_test_2(multithreading: bool) {
    println!("run LinesNet test");
    for i in 1..12 {
        let n = 1 << i;
        LinesNetTest::run(n, OverlayRule::Intersect, 500.0, multithreading);
    }
}

fn run_test_3(multithreading: bool) {
    println!("run WindMill test");
    for i in 1..21 {
        let n = 1 << i;
        WindMillTest::run(n, OverlayRule::Difference, 100.0, multithreading)
    }
}

fn run_test_4(multithreading: bool) {
    println!("run Windows test");
    for i in 1..12 {
        let n = 1 << i;
        WindowsTest::run(n, OverlayRule::Difference, 100.0, multithreading)
    }
}

fn run_test_5(multithreading: bool) {
    println!("run NestedSquares test");
    for i in 1..19 {
        let n = 1 << i;
        CrossTest::run(n, OverlayRule::Xor, 100.0, multithreading)
    }
}