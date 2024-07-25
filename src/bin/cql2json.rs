use cql2::parse_stdin;

fn main() {
    let parsed = parse_stdin();
    println!("{}", parsed.as_json().unwrap());
}
