use clap::Parser;
use cql2_cli::Cli;

fn main() {
    Cli::parse().run()
}
