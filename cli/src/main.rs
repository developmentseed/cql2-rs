use clap::{ArgAction, Parser, ValueEnum};
use cql2::{Expr, Validator};
use std::io::Read;

#[derive(Debug, Parser)]
struct Cli {
    /// The input CQL2
    ///
    /// If not provided, or `-`, the CQL2 will be read from standard input. The
    /// type (json or text) will be auto-detected. To specify a format, use
    /// --input-format.
    input: Option<String>,

    /// The input format.
    ///
    /// If not provided, the format will be auto-detected from the input.
    #[arg(short, long)]
    input_format: Option<InputFormat>,

    /// The output format.
    ///
    /// If not provided, the format will be the same as the input.
    #[arg(short, long)]
    output_format: Option<OutputFormat>,

    /// Validate the CQL2
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    validate: bool,

    /// Verbosity.
    ///
    /// Provide this argument several times to turn up the chatter.
    #[arg(short, long, action = ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug, ValueEnum, Clone)]
enum InputFormat {
    /// cql2-json
    Json,

    /// cql2-text
    Text,
}

#[derive(Debug, ValueEnum, Clone)]
enum OutputFormat {
    /// cql2-json, pretty-printed
    JsonPretty,

    /// cql2-json, compact
    Json,

    /// cql2-text
    Text,

    /// SQL
    Sql,
}

fn main() {
    let cli = Cli::parse();
    let input = cli
        .input
        .and_then(|input| if input == "-" { None } else { Some(input) })
        .unwrap_or_else(read_stdin);
    let input_format = cli.input_format.unwrap_or_else(|| {
        if input.starts_with('{') {
            InputFormat::Json
        } else {
            InputFormat::Text
        }
    });
    let expr: Expr = match input_format {
        InputFormat::Json => cql2::parse_json(&input).unwrap(),
        InputFormat::Text => match cql2::parse_text(&input) {
            Ok(expr) => expr,
            Err(err) => {
                eprintln!("[ERROR] Parsing error: {input}");
                eprintln!("{err}");
                std::process::exit(1)
            }
        },
    };
    if cli.validate {
        let validator = Validator::new().unwrap();
        let value = serde_json::to_value(&expr).unwrap();
        if let Err(error) = validator.validate(&value) {
            eprintln!("[ERROR] Invalid CQL2: {input}");
            match cli.verbose {
                0 => eprintln!("For more detailed validation information, use -v"),
                1 => eprintln!("For more detailed validation information, use -vv\n{error}"),
                2 => eprintln!("For more detailed validation information, use -vvv\n{error:#}"),
                _ => {
                    let detailed_output = error.detailed_output();
                    eprintln!("{detailed_output:#}");
                }
            }
            std::process::exit(1)
        }
    }
    let output_format = cli.output_format.unwrap_or(match input_format {
        InputFormat::Json => OutputFormat::Json,
        InputFormat::Text => OutputFormat::Text,
    });
    match output_format {
        OutputFormat::JsonPretty => serde_json::to_writer_pretty(std::io::stdout(), &expr).unwrap(),
        OutputFormat::Json => serde_json::to_writer(std::io::stdout(), &expr).unwrap(),
        OutputFormat::Text => print!("{}", expr.to_text().unwrap()),
        OutputFormat::Sql => {
            serde_json::to_writer_pretty(std::io::stdout(), &expr.to_sql().unwrap()).unwrap()
        }
    }
    println!()
}

fn read_stdin() -> String {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf).unwrap();
    buf
}
