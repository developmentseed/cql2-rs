use cql2::parse_stdin;

fn main() {
    let parsed = parse_stdin().unwrap();
    println!("{}", parsed.to_json().unwrap());
}
