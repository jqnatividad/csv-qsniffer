# csv-qsniffer

A high-performance CSV dialect detection library for Rust, implementing the Table Uniformity Method (TUM) for superior accuracy in CSV format detection.

## Overview

`csv-qsniffer` is a Rust implementation of the advanced CSV dialect detection approach described by Wilfredo Garcia (@ws-garcia), which significantly outperforms existing solutions like CleverCSV and Python's csv.Sniffer. The library uses table uniformity measurements combined with data type inference to detect the most likely CSV dialect configuration.

## Performance

Based on the original research, this approach achieves:
- **92.60% F1 score** vs CleverCSV's 84.25% and csv.Sniffer's 80.49%
- **90.04% success ratio** across diverse datasets
- Superior handling of edge cases and malformed CSV files

## Features

- **High Accuracy**: Uses the Table Uniformity Method for superior dialect detection
- **Comprehensive Data Type Detection**: Recognizes integers, floats, dates, emails, URLs, currencies, and more
- **Multiple Delimiter Support**: Automatically detects commas, semicolons, tabs, pipes, and spaces
- **Quote Character Detection**: Handles various quote characters and escaping scenarios
- **Header Detection**: Intelligently determines if the first row contains headers
- **Flexible Input**: Supports both string and reader-based input
- **Command Line Interface**: Includes a CLI tool for easy CSV dialect detection
- **Multiple Output Formats**: Human-readable, JSON, and CSV output formats

### Feature Flags

This crate supports the following feature flags:

- **`cli`** (optional): Enables the command-line interface and includes CLI dependencies (`clap` and `serde_json`). This feature is required to build the binary.

By default, no optional features are enabled, keeping the library lightweight with minimal dependencies for library users.

## Installation

### As a Library

Add this to your `Cargo.toml`:

```toml
[dependencies]
csv-qsniffer = "0.1.0"
```

### As a CLI Tool

Install the CLI tool using cargo with the `cli` feature:

```bash
cargo install csv-qsniffer --features cli
```

Or build from source:

```bash
git clone https://github.com/jqnatividad/csv-qsniffer
cd csv-qsniffer
cargo build --release --features cli
# Binary will be at target/release/csv-qsniffer
```

**Note**: The CLI functionality is gated behind the `cli` feature flag to keep the library lightweight for users who only need the detection functionality. When using as a library, the CLI dependencies (`clap` and `serde_json`) are not included unless explicitly enabled.

## Quick Start

### Library Usage

```rust
use csv_qsniffer::Sniffer;

let csv_data = "name,age,city\nJohn,25,NYC\nJane,30,LA";
let sniffer = Sniffer::new();
let dialect = sniffer.sniff_from_string(csv_data).unwrap();

println!("Delimiter: {:?}", dialect.delimiter as char);
println!("Quote char: {:?}", dialect.quote_char.map(|c| c as char));
println!("Has headers: {}", dialect.has_headers);
```

### CLI Usage

```bash
# Analyze a CSV file
csv-qsniffer data.csv

# Read from stdin
cat data.csv | csv-qsniffer

# Output as JSON
csv-qsniffer data.csv --format json

# Output as CSV (delimiter,quote_char,has_headers,escape)
csv-qsniffer data.csv --format csv

# Verbose output with detailed information
csv-qsniffer data.csv --verbose

# Analyze only first 100 rows
csv-qsniffer data.csv --max-rows 100
```

## CLI Reference

The `csv-qsniffer` command-line tool provides an easy way to detect CSV dialects from the command line.

### Usage

```
csv-qsniffer [OPTIONS] [FILE]
```

### Arguments

- `[FILE]` - Input CSV file (use '-' for stdin, or omit to read from stdin)

### Options

- `-f, --format <FORMAT>` - Output format: `human` (default), `json`, or `csv`
- `--max-rows <MAX_ROWS>` - Maximum number of rows to analyze (default: 1000)
- `--min-rows <MIN_ROWS>` - Minimum number of rows required for analysis (default: 2)
- `-v, --verbose` - Show detailed analysis information
- `-h, --help` - Print help information
- `-V, --version` - Print version information

### Output Formats

#### Human-readable (default)
```
CSV Dialect Detection Results:
==============================
Delimiter: ',' (44)
Quote character: '"'
Escape character: None
Has headers: true
```

#### JSON
```json
{
  "delimiter": ",",
  "delimiter_byte": 44,
  "quote_char": "\"",
  "quote_char_byte": 34,
  "escape": null,
  "escape_byte": null,
  "has_headers": true,
  "terminator": "LF",
  "quoting": "Necessary"
}
```

#### CSV
```
,,",true,
```
(Format: delimiter,quote_char,has_headers,escape)

### Examples

```bash
# Basic usage
csv-qsniffer sales_data.csv

# Pipe from another command
curl -s https://example.com/data.csv | csv-qsniffer

# JSON output for programmatic use
csv-qsniffer data.csv --format json | jq '.delimiter'

# Quick CSV format for shell scripts
DELIMITER=$(csv-qsniffer data.csv --format csv | cut -d, -f1)
```

## Usage Examples

### Basic CSV Detection

```rust
use csv_qsniffer::Sniffer;

let csv_data = "name,age,city\nJohn,25,NYC\nJane,30,LA\nBob,35,SF";
let sniffer = Sniffer::new();
let dialect = sniffer.sniff_from_string(csv_data)?;

assert_eq!(dialect.delimiter, b',');
assert_eq!(dialect.quote_char, Some(b'"'));
assert!(dialect.has_headers);
```

### Different Delimiters

```rust
// Semicolon-separated
let csv_data = "name;age;city\nJohn;25;NYC\nJane;30;LA";
let dialect = sniffer.sniff_from_string(csv_data)?;
assert_eq!(dialect.delimiter, b';');

// Tab-separated
let csv_data = "name\tage\tcity\nJohn\t25\tNYC\nJane\t30\tLA";
let dialect = sniffer.sniff_from_string(csv_data)?;
assert_eq!(dialect.delimiter, b'\t');
```

### Using Reader Interface

```rust
use std::io::Cursor;

let csv_data = "name,age,city\nJohn,25,NYC\nJane,30,LA";
let cursor = Cursor::new(csv_data);
let dialect = sniffer.sniff(cursor)?;
```

### Complex CSV with Quotes

```rust
let csv_data = r#"name,description,price
"John Doe","A person with, comma",25.50
"Jane Smith","Another ""quoted"" person",30.75"#;

let dialect = sniffer.sniff_from_string(csv_data)?;
assert_eq!(dialect.delimiter, b',');
assert_eq!(dialect.quote_char, Some(b'"'));
```

## API Reference

### `Sniffer`

The main dialect detection engine.

#### Methods

- `new() -> Self`: Create a new sniffer with default settings
- `sniff<R: BufRead>(&self, reader: R) -> Result<Dialect, SnifferError>`: Detect dialect from a reader
- `sniff_from_string(&self, data: &str) -> Result<Dialect, SnifferError>`: Detect dialect from string data

#### Configuration

- `max_rows`: Maximum number of rows to analyze (default: 1000)
- `min_rows`: Minimum number of rows required (default: 2)

### `Dialect`

Represents a detected CSV dialect configuration.

#### Fields

- `delimiter: u8`: Field delimiter (e.g., `,`, `;`, `\t`)
- `quote_char: Option<u8>`: Quote character (e.g., `"`, `'`)
- `escape: Option<u8>`: Escape character
- `has_headers: bool`: Whether the first row contains headers
- `terminator: csv::Terminator`: Line terminator
- `quoting: csv::QuoteStyle`: Quoting style

### `DataType`

Enumeration of detectable data types:

- `Integer`: Whole numbers
- `Float`: Decimal numbers
- `Boolean`: True/false values
- `Date`: Date values
- `Time`: Time values
- `DateTime`: Combined date and time
- `Email`: Email addresses
- `Url`: Web URLs
- `Phone`: Phone numbers
- `Currency`: Monetary values
- `Percentage`: Percentage values
- `Text`: General text
- `Empty`: Empty fields

## Algorithm Details

The library implements the Table Uniformity Method (TUM) which:

1. **Generates Potential Dialects**: Creates candidate configurations based on common delimiters and quote characters
2. **Parses with Each Dialect**: Attempts to parse the CSV data using each potential dialect
3. **Analyzes Data Types**: Uses regex patterns to detect data types in each column
4. **Calculates Table Uniformity**: Scores each table based on type consistency and structure
5. **Selects Best Dialect**: Returns the dialect that produces the highest uniformity score

### Scoring Factors

- **Type Consistency**: Columns with uniform data types score higher
- **Type Weights**: Structured types (numbers, dates) receive higher weights than text
- **Empty Field Penalty**: Tables with many empty fields are penalized
- **Row Consistency**: Consistent row lengths are rewarded

## Error Handling

The library uses the `SnifferError` enum for error handling:

- `CsvError`: Errors from the underlying CSV parser
- `IoError`: I/O related errors
- `NoValidDialect`: No suitable dialect could be detected
- `InvalidInput`: Input data is invalid or insufficient

## Performance Considerations

- The library analyzes up to 1000 rows by default for performance
- Minimum 2 rows required for reliable detection
- Regex-based type detection is optimized for common patterns
- Memory usage scales linearly with input size

## Integration with qsv

This library is designed for integration with the [qsv](https://github.com/dathere/qsv) toolkit, providing enhanced CSV dialect detection capabilities for data processing workflows.

## Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

## License

This project is licensed under the MIT OR Apache-2.0 license.

## Acknowledgments

- Based on the research and Python implementation by [ws-garcia](https://github.com/ws-garcia/CSVsniffer)
- Inspired by the Table Uniformity Method research paper
- Designed for integration with the [qsv](https://github.com/dathere/qsv) project

## References

- [Original CSVsniffer Python Implementation](https://github.com/ws-garcia/CSVsniffer/tree/main/python/src)
- [qsv GitHub Issue #2247](https://github.com/dathere/qsv/issues/2247)
- Garcia, W. (2024). "[Detecting CSV file dialects by table uniformity measurement and data type inference](https://journals.sagepub.com/doi/10.3233/DS-240062)". Data Science, 7(2), 55-72. DOI: 10.3233/DS-240062