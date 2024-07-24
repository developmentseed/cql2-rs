use cql2::parse;
use std::io::{self, BufRead};
fn main() -> io::Result<()> {
    for line in io::stdin().lock().lines() {
        let parsed = parse(&line?);

        println!("Parsed: {:#?}", &parsed);
        println!("{}", parsed.as_json());

        parsed.validate();
    }
    Ok(())
}
