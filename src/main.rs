use cql2::Validator;
use std::io::BufRead;

fn main() {
    let debug_level: u8 = std::env::var("CQL2_DEBUG_LEVEL")
        .map(|s| {
            s.parse()
                .unwrap_or_else(|_| panic!("CQL2_DEBUG_LEVEL should be an integer: {}", s))
        })
        .unwrap_or(1);
    let validator = Validator::new().unwrap();

    let mut ok = true;
    for line in std::io::stdin().lock().lines() {
        let parsed = cql2::parse(&line.unwrap());
        println!("Parsed: {:#?}", &parsed);
        println!("{}", parsed.to_json().unwrap());
        let value = serde_json::to_value(parsed).unwrap();

        if let Err(err) = validator.validate(&value) {
            match debug_level {
                0 => println!("-----------\nCQL2 Is Invalid!\n---------------"),
                1 => println!("-----------\n{err}\n---------------"),
                2 => println!("-----------\n{err:#}\n---------------"),
                _ => {
                    let detailed_output = err.detailed_output();
                    println!("-----------\n{detailed_output:#}\n---------------");
                }
            }
            ok = false;
        }
    }

    if !ok {
        std::process::exit(1);
    }
}
