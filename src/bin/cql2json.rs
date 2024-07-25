use cql2_rs::parse;
use std::io;
use std::env;
use atty::Stream;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut buffer = String::new();
    if args.len() >= 2 {
        buffer = args[1].clone();
    }
    else if atty::isnt(Stream::Stdin){
        io::stdin().read_line(&mut buffer).unwrap();
    }
    else {
        println!("Enter CQL2 as Text or JSON, then hit return");
        io::stdin().read_line(&mut buffer).unwrap();
    }
    let parsed = parse(&buffer);
    if args.len() == 3 && args[2] == "pretty" {
        println!("{}", parsed.as_json_pretty());
    }
    else {
        println!("{}", parsed.as_json());
    }

    if parsed.validate() {
        return 0.into()
    }
    1.into()
}
