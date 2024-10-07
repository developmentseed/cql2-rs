use cql2::parse_stdin;

fn main() {
    if let Ok(parsed) = parse_stdin() {
        println!("{}", parsed.to_json().unwrap());
    } else {
        std::process::exit(1)
    }
}
