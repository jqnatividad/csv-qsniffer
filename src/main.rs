#![cfg(feature = "cli")]

use clap::{Parser, ValueEnum};
use csv_qsniffer::{Dialect, Sniffer};
use serde_json;
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "csv-qsniffer")]
#[command(about = "A CSV dialect detection tool using Table Uniformity Method")]
#[command(version)]
struct Cli {
    /// Input CSV file (use '-' for stdin)
    #[arg(value_name = "FILE")]
    input: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Human)]
    format: OutputFormat,

    /// Maximum number of rows to analyze
    #[arg(long, default_value_t = 1000)]
    max_rows: usize,

    /// Minimum number of rows required for analysis
    #[arg(long, default_value_t = 2)]
    min_rows: usize,

    /// Show detailed analysis information
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    /// Human-readable output
    Human,
    /// JSON output
    Json,
    /// CSV output (delimiter,quote_char,has_headers,escape)
    Csv,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Create sniffer with custom settings
    let mut sniffer = Sniffer::new();
    sniffer.max_rows = cli.max_rows;
    sniffer.min_rows = cli.min_rows;

    // Read input data
    let input_data = match &cli.input {
        Some(path) if path.to_str() == Some("-") => {
            if cli.verbose {
                eprintln!("Reading from stdin...");
            }
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
        Some(path) => {
            if cli.verbose {
                eprintln!("Reading from file: {}", path.display());
            }
            let file = File::open(path)?;
            let mut reader = BufReader::new(file);
            let mut buffer = String::new();
            reader.read_to_string(&mut buffer)?;
            buffer
        }
        None => {
            if cli.verbose {
                eprintln!("Reading from stdin...");
            }
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    if input_data.trim().is_empty() {
        eprintln!("Error: No input data provided");
        std::process::exit(1);
    }

    // Detect dialect
    let dialect = match sniffer.sniff_from_string(&input_data) {
        Ok(dialect) => dialect,
        Err(e) => {
            eprintln!("Error detecting CSV dialect: {}", e);
            std::process::exit(1);
        }
    };

    // Output results
    match cli.format {
        OutputFormat::Human => print_human_readable(&dialect, cli.verbose),
        OutputFormat::Json => print_json(&dialect)?,
        OutputFormat::Csv => print_csv(&dialect),
    }

    Ok(())
}

fn print_human_readable(dialect: &Dialect, verbose: bool) {
    println!("CSV Dialect Detection Results:");
    println!("==============================");

    let delimiter_display = match dialect.delimiter {
        b'\t' => "\\t (tab)".to_string(),
        b' ' => "\\s (space)".to_string(),
        b => format!("'{}' ({})", b as char, b),
    };
    println!("Delimiter: {}", delimiter_display);

    match dialect.quote_char {
        Some(quote) => println!("Quote character: '{}'", quote as char),
        None => println!("Quote character: None"),
    }

    match dialect.escape {
        Some(escape) => println!("Escape character: '{}'", escape as char),
        None => println!("Escape character: None"),
    }

    println!("Has headers: {}", dialect.has_headers);

    if verbose {
        println!("Line terminator: {:?}", dialect.terminator);
        println!("Quoting style: {:?}", dialect.quoting);
    }
}

fn print_json(dialect: &Dialect) -> Result<(), Box<dyn std::error::Error>> {
    let json_output = serde_json::json!({
        "delimiter": dialect.delimiter as char,
        "delimiter_byte": dialect.delimiter,
        "quote_char": dialect.quote_char.map(|c| c as char),
        "quote_char_byte": dialect.quote_char,
        "escape": dialect.escape.map(|c| c as char),
        "escape_byte": dialect.escape,
        "has_headers": dialect.has_headers,
        "terminator": match dialect.terminator {
            csv::Terminator::CRLF => "CRLF",
            csv::Terminator::Any(b'\n') => "LF",
            csv::Terminator::Any(b'\r') => "CR",
            _ => "Other",
        },
        "quoting": match dialect.quoting {
            csv::QuoteStyle::Always => "Always",
            csv::QuoteStyle::Necessary => "Necessary",
            csv::QuoteStyle::NonNumeric => "NonNumeric",
            csv::QuoteStyle::Never => "Never",
            _ => "Other",
        }
    });

    println!("{}", serde_json::to_string_pretty(&json_output)?);
    Ok(())
}

fn print_csv(dialect: &Dialect) {
    let delimiter = dialect.delimiter as char;
    let quote_char = dialect.quote_char.map(|c| c as char).unwrap_or('\0');
    let escape = dialect.escape.map(|c| c as char).unwrap_or('\0');

    println!(
        "{},{},{},{}",
        delimiter,
        if quote_char == '\0' {
            "".to_string()
        } else {
            quote_char.to_string()
        },
        dialect.has_headers,
        if escape == '\0' {
            "".to_string()
        } else {
            escape.to_string()
        }
    );
}
