use cql2_rs::parse;
use std::io::{self, BufRead};
fn main() -> io::Result<()> {
    for line in io::stdin().lock().lines() {
        let parsed = parse(&line?);

        println!("Parsed: {}", parsed.as_cql2_text());
        let sql = parsed.as_sql();
        println!("SQL: {:#?}", sql);
        println!("{}", parsed.as_json());

        parsed.validate();
    }
    Ok(())
}
