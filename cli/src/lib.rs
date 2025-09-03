use anyhow::{anyhow, Result};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use cql2::{Expr, ToSqlAst, Validator};
use std::io::Read;

/// The CQL2 command-line interface.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Subcommand to run (e.g., Filter)
    #[command(subcommand)]
    cmd: Option<Commands>,

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

    /// Reduce the CQL2
    #[arg(long, default_value_t = false, action = ArgAction::Set)]
    reduce: bool,

    /// Verbosity.
    ///
    /// Provide this argument several times to turn up the chatter.
    #[arg(short, long, action = ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Filter an NDJSON file by a CQL2 expression
    Filter {
        /// The CQL2 expression (text or JSON)
        expression: String,
        /// Input NDJSON file path
        #[arg(long)]
        file: String,
    },
}

/// The input CQL2 format.
#[derive(Debug, ValueEnum, Clone)]
pub enum InputFormat {
    /// cql2-json
    Json,

    /// cql2-text
    Text,
}

/// The output CQL2 format.
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

impl Cli {
    /// Runs the cli.
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2_cli::Cli;
    /// use clap::Parser;
    ///
    /// let cli = Cli::try_parse_from(&["cql2", "landsat:scene_id = 'LC82030282019133LGN00'"]).unwrap();
    /// cli.run();
    /// ```
    pub fn run(self) {
        if let Err(err) = self.run_inner() {
            eprintln!("{}", err);
            std::process::exit(1)
        }
    }

    pub fn run_inner(self) -> Result<()> {
        if let Some(Commands::Filter { expression, file }) = self.cmd {
            use std::fs::File;
            use std::io::{BufRead, BufReader};
            let expr: Expr = if expression.trim_start().starts_with('{') {
                cql2::parse_json(&expression)?
            } else {
                cql2::parse_text(&expression)?
            };
            let file = File::open(file)?;
            let reader = BufReader::new(file);
            reader
                .lines()
                .map(|line| {
                    let line = line?;
                    Ok(serde_json::from_str(&line)?)
                })
                .collect::<Result<Vec<_>, anyhow::Error>>()?
                .into_iter()
                .filter_map(|value| {
                    expr.filter(&[value])
                        .ok()
                        .and_then(|mut v| v.pop().cloned())
                })
                .for_each(|v| println!("{}", serde_json::to_string(&v).unwrap()));
            return Ok(());
        }
        let input = self
            .input
            .and_then(|input| if input == "-" { None } else { Some(input) })
            .map(Ok)
            .unwrap_or_else(read_stdin)?;
        let input_format = self.input_format.unwrap_or_else(|| {
            if input.starts_with('{') {
                InputFormat::Json
            } else {
                InputFormat::Text
            }
        });
        let mut expr: Expr = match input_format {
            InputFormat::Json => cql2::parse_json(&input)?,
            InputFormat::Text => match cql2::parse_text(&input) {
                Ok(expr) => expr,
                Err(err) => {
                    return Err(anyhow!("[ERROR] Parsing error: {input}\n{err}"));
                }
            },
        };
        if self.reduce {
            expr = expr.reduce(None)?;
        }
        if self.validate {
            let validator = Validator::new().unwrap();
            let value = serde_json::to_value(&expr).unwrap();
            if let Err(error) = validator.validate(&value) {
                return Err(anyhow!(
                    "[ERROR] Invalid CQL2: {input}\n{}",
                    match self.verbose {
                        0 => "For more detailed validation information, use -v".to_string(),
                        1 => format!("For more detailed validation information, use -vv\n{error}"),
                        _ => format!("{error:#}"),
                    }
                ));
            }
        }
        let output_format = self.output_format.unwrap_or(match input_format {
            InputFormat::Json => OutputFormat::Json,
            InputFormat::Text => OutputFormat::Text,
        });
        match output_format {
            OutputFormat::JsonPretty => serde_json::to_writer_pretty(std::io::stdout(), &expr)?,
            OutputFormat::Json => serde_json::to_writer(std::io::stdout(), &expr)?,
            OutputFormat::Text => print!("{}", expr.to_text()?),
            OutputFormat::Sql => {
                let sql_ast = expr.to_sql_ast()?;
                println!("{}", sql_ast);
            }
        }
        println!();
        Ok(())
    }
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}
