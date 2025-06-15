//! # csv-qsniffer
//!
//! A CSV dialect detection library using the Table Uniformity Method (TUM).
//!
//! This library implements the approach described by ws-garcia, which outperforms
//! existing solutions like `CleverCSV` and csv.Sniffer by using table uniformity
//! measurements to detect the best CSV dialect.
//!
//! ## Example
//!
//! ```rust
//! use csv_qsniffer::{Sniffer, Dialect};
//!
//! let csv_data = "name,age,city\nJohn,25,NYC\nJane,30,LA";
//! let sniffer = Sniffer::new();
//! let dialect = sniffer.sniff(csv_data.as_bytes()).unwrap();
//!
//! assert_eq!(dialect.delimiter, b',');
//! assert_eq!(dialect.quote_char, Some(b'"'));
//! ```

use csv::{ReaderBuilder, StringRecord};
use regex::Regex;
use std::collections::HashMap;
use std::io::{BufRead, Cursor};
use std::sync::OnceLock;
use thiserror::Error;

/// Errors that can occur during CSV dialect detection
#[derive(Error, Debug)]
pub enum SnifferError {
    #[error("CSV parsing error: {0}")]
    CsvError(#[from] csv::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("No valid dialect found")]
    NoValidDialect,
    #[error("Invalid input data")]
    InvalidInput,
}

/// Data types that can be detected in CSV fields
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataType {
    Integer,
    Float,
    Boolean,
    Date,
    Time,
    DateTime,
    Email,
    Url,
    Phone,
    Currency,
    Percentage,
    Text,
    Empty,
}

/// Table structure for uniformity analysis
#[derive(Debug)]
struct Table {
    records: Vec<StringRecord>,
    column_types: Vec<Vec<DataType>>,
    num_columns: usize,
    num_rows: usize,
}

/// Global static regex cache - compiled once and reused across all Sniffer instances
static TYPE_REGEXES: OnceLock<HashMap<DataType, Regex>> = OnceLock::new();

/// Initialize the global regex cache
fn get_type_regexes() -> &'static HashMap<DataType, Regex> {
    TYPE_REGEXES.get_or_init(|| {
        let mut type_regexes = HashMap::new();

        // Integer pattern
        type_regexes.insert(DataType::Integer, Regex::new(r"^[+-]?\d+$").unwrap());

        // Float pattern
        type_regexes.insert(
            DataType::Float,
            Regex::new(r"^[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?$").unwrap(),
        );

        // Boolean pattern
        type_regexes.insert(
            DataType::Boolean,
            Regex::new(r"^(?i)(true|false|yes|no|y|n|1|0|on|off)$").unwrap(),
        );

        // Date patterns (various formats)
        type_regexes.insert(
            DataType::Date,
            Regex::new(r"^\d{1,4}[-/]\d{1,2}[-/]\d{1,4}$").unwrap(),
        );

        // Time pattern
        type_regexes.insert(
            DataType::Time,
            Regex::new(r"^\d{1,2}:\d{2}(:\d{2})?(\s?(AM|PM))?$").unwrap(),
        );

        // DateTime pattern
        type_regexes.insert(
            DataType::DateTime,
            Regex::new(r"^\d{1,4}[-/]\d{1,2}[-/]\d{1,4}\s+\d{1,2}:\d{2}(:\d{2})?").unwrap(),
        );

        // Email pattern
        type_regexes.insert(
            DataType::Email,
            Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap(),
        );

        // URL pattern
        type_regexes.insert(
            DataType::Url,
            Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap(),
        );

        // Phone pattern (basic)
        type_regexes.insert(
            DataType::Phone,
            Regex::new(r"^[\+]?[\d\s\-\(\)\.]{7,15}$").unwrap(),
        );

        // Currency pattern
        type_regexes.insert(
            DataType::Currency,
            Regex::new(r"^[\$£€¥]?[+-]?\d{1,3}(,\d{3})*(\.\d{2})?[\$£€¥]?$").unwrap(),
        );

        // Percentage pattern
        type_regexes.insert(
            DataType::Percentage,
            Regex::new(r"^[+-]?\d+(\.\d+)?%$").unwrap(),
        );

        type_regexes
    })
}

/// Main CSV dialect detection engine
pub struct Sniffer {
    /// Maximum number of rows to analyze for dialect detection
    pub max_rows: usize,
    /// Minimum number of rows required for analysis
    pub min_rows: usize,
}

impl Default for Sniffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Sniffer {
    /// Create a new CSV sniffer with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_rows: 1000,
            min_rows: 2,
        }
    }

    /// Detect the most likely CSV dialect for the given data
    pub fn sniff<R: BufRead>(&self, reader: R) -> Result<Dialect, SnifferError> {
        // Read sample data
        let mut sample_data = String::new();
        let mut lines_read = 0;

        for line in reader.lines() {
            if lines_read >= self.max_rows {
                break;
            }
            sample_data.push_str(&line?);
            sample_data.push('\n');
            lines_read += 1;
        }

        if lines_read < self.min_rows {
            return Err(SnifferError::InvalidInput);
        }

        self.sniff_from_string(&sample_data)
    }

    /// Detect dialect from string data
    pub fn sniff_from_string(&self, data: &str) -> Result<Dialect, SnifferError> {
        let potential_dialects = self.generate_potential_dialects(data);
        let mut best_dialect = None;
        let mut best_score = f64::NEG_INFINITY;

        for dialect in potential_dialects {
            if let Ok(table) = self.parse_with_dialect(data, &dialect) {
                let score = self.calculate_table_uniformity(&table);
                if score > best_score {
                    best_score = score;
                    best_dialect = Some(dialect);
                }
            }
        }

        best_dialect.ok_or(SnifferError::NoValidDialect)
    }

    /// Generate potential CSV dialects based on data analysis
    fn generate_potential_dialects(&self, data: &str) -> Vec<Dialect> {
        let mut dialects = Vec::new();

        // Common delimiters to test
        let delimiters = [b',', b';', b'\t', b'|', b' '];

        // Common quote characters
        let quote_chars = [Some(b'"'), Some(b'\''), None];

        // Analyze first few lines to get hints
        let lines: Vec<&str> = data.lines().take(10).collect();

        for &delimiter in &delimiters {
            for &quote_char in &quote_chars {
                // Skip combinations that don't make sense
                if delimiter == b' ' && quote_char.is_none() {
                    continue; // Space delimiter usually needs quotes
                }

                let dialect = Dialect {
                    delimiter,
                    quote_char,
                    escape: None,
                    has_headers: self.detect_headers(&lines, delimiter),
                    terminator: csv::Terminator::Any(b'\n'),
                    quoting: if quote_char.is_some() {
                        csv::QuoteStyle::Necessary
                    } else {
                        csv::QuoteStyle::Never
                    },
                };

                dialects.push(dialect);
            }
        }

        dialects
    }

    /// Detect if the CSV likely has headers
    fn detect_headers(&self, lines: &[&str], delimiter: u8) -> bool {
        if lines.len() < 2 {
            return false;
        }

        let first_line = lines[0];
        let second_line = lines[1];

        // Count fields in first two lines
        let first_fields: Vec<&str> = first_line.split(delimiter as char).collect();
        let second_fields: Vec<&str> = second_line.split(delimiter as char).collect();

        if first_fields.len() != second_fields.len() {
            return false;
        }

        // Check if first line looks like headers (more text, less numbers)
        let first_numeric_count = first_fields
            .iter()
            .filter(|field| self.is_numeric(field.trim()))
            .count();

        let second_numeric_count = second_fields
            .iter()
            .filter(|field| self.is_numeric(field.trim()))
            .count();

        // Headers typically have fewer numeric values
        first_numeric_count < second_numeric_count
    }

    /// Check if a field looks numeric
    fn is_numeric(&self, field: &str) -> bool {
        if field.is_empty() {
            return false;
        }

        let regexes = get_type_regexes();
        regexes.get(&DataType::Integer).unwrap().is_match(field)
            || regexes.get(&DataType::Float).unwrap().is_match(field)
    }

    /// Parse CSV data with a specific dialect
    fn parse_with_dialect(&self, data: &str, dialect: &Dialect) -> Result<Table, SnifferError> {
        let mut builder = ReaderBuilder::new();
        builder.delimiter(dialect.delimiter);

        if let Some(quote) = dialect.quote_char {
            builder.quote(quote);
        } else {
            builder.quoting(false);
        }

        builder.has_headers(dialect.has_headers);
        builder.terminator(dialect.terminator);

        let mut reader = builder.from_reader(Cursor::new(data));
        let mut records = Vec::new();
        let mut num_columns = 0;

        // Read all records
        for result in reader.records() {
            let record = result?;
            if num_columns == 0 {
                num_columns = record.len();
            } else if record.len() != num_columns {
                // Inconsistent column count - this dialect might not be correct
                continue;
            }
            records.push(record);
        }

        if records.is_empty() {
            return Err(SnifferError::InvalidInput);
        }

        // Analyze data types for each column
        let mut column_types = vec![Vec::new(); num_columns];

        for record in &records {
            for (col_idx, field) in record.iter().enumerate() {
                if col_idx < num_columns {
                    let data_type = self.detect_data_type(field);
                    column_types[col_idx].push(data_type);
                }
            }
        }

        let num_rows = records.len();

        Ok(Table {
            records,
            column_types,
            num_columns,
            num_rows,
        })
    }

    /// Detect the data type of a field
    fn detect_data_type(&self, field: &str) -> DataType {
        let trimmed = field.trim();

        if trimmed.is_empty() {
            return DataType::Empty;
        }

        let regexes = get_type_regexes();

        // Check each data type in order of specificity
        let type_order = [
            DataType::Boolean,
            DataType::Integer,
            DataType::Float,
            DataType::DateTime,
            DataType::Date,
            DataType::Time,
            DataType::Email,
            DataType::Url,
            DataType::Phone,
            DataType::Currency,
            DataType::Percentage,
        ];

        for data_type in &type_order {
            #[allow(clippy::collapsible_if)]
            if let Some(regex) = regexes.get(data_type) {
                if regex.is_match(trimmed) {
                    return data_type.clone();
                }
            }
        }

        DataType::Text
    }

    /// Calculate table uniformity score using the Table Uniformity Method
    fn calculate_table_uniformity(&self, table: &Table) -> f64 {
        if table.num_rows == 0 || table.num_columns == 0 {
            return f64::NEG_INFINITY;
        }

        let mut total_score = 0.0;
        let mut valid_columns = 0;

        for col_idx in 0..table.num_columns {
            let column_types = &table.column_types[col_idx];
            if column_types.is_empty() {
                continue;
            }

            // Calculate type consistency for this column
            let type_counts = self.count_types(column_types);
            let column_score = self.calculate_column_uniformity(&type_counts, column_types.len());

            total_score += column_score;
            valid_columns += 1;
        }

        if valid_columns == 0 {
            return f64::NEG_INFINITY;
        }

        // Average uniformity across all columns
        let avg_uniformity = total_score / f64::from(valid_columns);

        // Bonus for consistent row length
        let row_consistency_bonus = 1.0; // All rows have same length if we got here

        // Penalty for too many empty fields
        let empty_penalty = self.calculate_empty_penalty(table);

        avg_uniformity.mul_add(row_consistency_bonus, -empty_penalty)
    }

    /// Count occurrences of each data type in a column
    fn count_types(&self, types: &[DataType]) -> HashMap<DataType, usize> {
        let mut counts = HashMap::new();
        for data_type in types {
            *counts.entry(data_type.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Calculate uniformity score for a single column
    fn calculate_column_uniformity(
        &self,
        type_counts: &HashMap<DataType, usize>,
        total_count: usize,
    ) -> f64 {
        if total_count == 0 {
            return 0.0;
        }

        // Find the most common type (excluding empty)
        let mut max_count = 0;
        let mut dominant_type = DataType::Text;

        for (data_type, &count) in type_counts {
            if *data_type != DataType::Empty && count > max_count {
                max_count = count;
                dominant_type = data_type.clone();
            }
        }

        // Calculate uniformity as ratio of dominant type
        let uniformity = max_count as f64 / total_count as f64;

        // Apply type-specific weights
        let type_weight = match dominant_type {
            DataType::Integer | DataType::Float => 1.2,
            DataType::Date | DataType::DateTime | DataType::Time => 1.1,
            DataType::Email | DataType::Url => 1.1,
            DataType::Boolean => 1.0,
            DataType::Text => 0.8,
            DataType::Empty => 0.1,
            _ => 1.0,
        };

        uniformity * type_weight
    }

    /// Calculate penalty for empty fields
    fn calculate_empty_penalty(&self, table: &Table) -> f64 {
        let total_fields = table.num_rows * table.num_columns;
        if total_fields == 0 {
            return 0.0;
        }

        let mut empty_count = 0;
        for column_types in &table.column_types {
            empty_count += column_types
                .iter()
                .filter(|&t| *t == DataType::Empty)
                .count();
        }

        let empty_ratio = empty_count as f64 / total_fields as f64;
        empty_ratio * 0.5 // Penalty factor
    }
}

/// Represents a CSV dialect configuration
#[derive(Debug, Clone)]
pub struct Dialect {
    /// Field delimiter (e.g., comma, semicolon, tab)
    pub delimiter: u8,
    /// Quote character (e.g., double quote, single quote)
    pub quote_char: Option<u8>,
    /// Escape character
    pub escape: Option<u8>,
    /// Whether to treat the first row as headers
    pub has_headers: bool,
    /// Line terminator
    pub terminator: csv::Terminator,
    /// Whether quotes are required around all fields
    pub quoting: csv::QuoteStyle,
}

impl PartialEq for Dialect {
    fn eq(&self, other: &Self) -> bool {
        self.delimiter == other.delimiter
            && self.quote_char == other.quote_char
            && self.escape == other.escape
            && self.has_headers == other.has_headers
        // Skip terminator and quoting comparison as they don't implement PartialEq
    }
}

impl Default for Dialect {
    fn default() -> Self {
        Self {
            delimiter: b',',
            quote_char: Some(b'"'),
            escape: None,
            has_headers: true,
            terminator: csv::Terminator::CRLF,
            quoting: csv::QuoteStyle::Necessary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_basic_csv_detection() {
        let csv_data = "name,age,city\nJohn,25,NYC\nJane,30,LA";
        let sniffer = Sniffer::new();
        let dialect = sniffer.sniff_from_string(csv_data).unwrap();

        assert_eq!(dialect.delimiter, b',');
        assert_eq!(dialect.quote_char, Some(b'"'));
        assert!(dialect.has_headers);
    }

    #[test]
    fn test_semicolon_delimiter() {
        let csv_data = "name;age;city\nJohn;25;NYC\nJane;30;LA";
        let sniffer = Sniffer::new();
        let dialect = sniffer.sniff_from_string(csv_data).unwrap();

        assert_eq!(dialect.delimiter, b';');
    }

    #[test]
    fn test_tab_delimiter() {
        let csv_data = "name\tage\tcity\nJohn\t25\tNYC\nJane\t30\tLA";
        let sniffer = Sniffer::new();
        let dialect = sniffer.sniff_from_string(csv_data).unwrap();

        assert_eq!(dialect.delimiter, b'\t');
    }

    #[test]
    fn test_data_type_detection() {
        let sniffer = Sniffer::new();

        assert_eq!(sniffer.detect_data_type("123"), DataType::Integer);
        assert_eq!(sniffer.detect_data_type("123.45"), DataType::Float);
        assert_eq!(sniffer.detect_data_type("true"), DataType::Boolean);
        assert_eq!(
            sniffer.detect_data_type("test@example.com"),
            DataType::Email
        );
        assert_eq!(
            sniffer.detect_data_type("https://example.com"),
            DataType::Url
        );
        assert_eq!(sniffer.detect_data_type("$123.45"), DataType::Currency);
        assert_eq!(sniffer.detect_data_type("50%"), DataType::Percentage);
        assert_eq!(sniffer.detect_data_type("hello world"), DataType::Text);
        assert_eq!(sniffer.detect_data_type(""), DataType::Empty);
    }

    #[test]
    fn test_header_detection() {
        let sniffer = Sniffer::new();

        // With headers
        let lines_with_headers = vec!["name,age,city", "John,25,NYC", "Jane,30,LA"];
        assert!(sniffer.detect_headers(&lines_with_headers, b','));

        // Without headers
        let lines_without_headers = vec!["John,25,NYC", "Jane,30,LA", "Bob,35,SF"];
        assert!(!sniffer.detect_headers(&lines_without_headers, b','));
    }

    #[test]
    fn test_reader_interface() {
        let csv_data = "name,age,city\nJohn,25,NYC\nJane,30,LA\nBob,35,SF\nAlice,28,Chicago";
        let cursor = Cursor::new(csv_data);
        let sniffer = Sniffer::new();
        let dialect = sniffer.sniff(cursor).unwrap();

        assert_eq!(dialect.delimiter, b',');
        assert!(dialect.has_headers);
    }

    #[test]
    fn test_complex_csv_with_quotes() {
        let csv_data = r#"name,description,price
"John Doe","A person with, comma",25.50
"Jane Smith","Another ""quoted"" person",30.75
"Bob Johnson","Simple person",40.00
"Alice Brown","Person with; semicolon",35.25"#;

        let sniffer = Sniffer::new();
        let dialect = sniffer.sniff_from_string(csv_data).unwrap();

        assert_eq!(dialect.delimiter, b',');
        assert_eq!(dialect.quote_char, Some(b'"'));
    }
}
