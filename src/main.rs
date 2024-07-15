
use std::io::{self, BufRead};
use cql2_rs::parse;
fn main() -> io::Result<()> {
    for line in io::stdin().lock().lines() {
        let parsed = parse(&line?);

        println!("Parsed: {:#?}", &parsed);
        println!("{}", parsed.as_json());

        parsed.validate();

    }
    Ok(())
}
