use csv_qsniffer::Sniffer;
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Basic CSV with comma delimiter
    let csv_data1 = "name,age,city\nJohn,25,NYC\nJane,30,LA\nBob,35,SF";
    let sniffer = Sniffer::new();
    let dialect1 = sniffer.sniff_from_string(csv_data1)?;

    println!("Example 1 - Basic CSV:");
    println!("  Delimiter: {:?}", dialect1.delimiter as char);
    println!("  Quote char: {:?}", dialect1.quote_char.map(|c| c as char));
    println!("  Has headers: {}", dialect1.has_headers);
    println!();

    // Example 2: Semicolon-separated values
    let csv_data2 = "name;age;city\nJohn;25;NYC\nJane;30;LA\nBob;35;SF";
    let dialect2 = sniffer.sniff_from_string(csv_data2)?;

    println!("Example 2 - Semicolon-separated:");
    println!("  Delimiter: {:?}", dialect2.delimiter as char);
    println!("  Quote char: {:?}", dialect2.quote_char.map(|c| c as char));
    println!("  Has headers: {}", dialect2.has_headers);
    println!();

    // Example 3: Tab-separated values
    let csv_data3 = "name\tage\tcity\nJohn\t25\tNYC\nJane\t30\tLA\nBob\t35\tSF";
    let dialect3 = sniffer.sniff_from_string(csv_data3)?;

    println!("Example 3 - Tab-separated:");
    let delimiter_display = if dialect3.delimiter == b'\t' {
        "\\t".to_string()
    } else {
        format!("{}", dialect3.delimiter as char)
    };
    println!("  Delimiter: {:?}", delimiter_display);
    println!("  Quote char: {:?}", dialect3.quote_char.map(|c| c as char));
    println!("  Has headers: {}", dialect3.has_headers);
    println!();

    // Example 4: CSV with quotes and embedded commas
    let csv_data4 = r#"name,description,price
"John Doe","A person with, comma",25.50
"Jane Smith","Another ""quoted"" person",30.75
"Bob Johnson","Simple person",40.00"#;
    let dialect4 = sniffer.sniff_from_string(csv_data4)?;

    println!("Example 4 - CSV with quotes:");
    println!("  Delimiter: {:?}", dialect4.delimiter as char);
    println!("  Quote char: {:?}", dialect4.quote_char.map(|c| c as char));
    println!("  Has headers: {}", dialect4.has_headers);
    println!();

    // Example 5: Using the reader interface
    let csv_data5 = "product|quantity|price\nApple|10|1.50\nBanana|20|0.75\nOrange|15|2.00";
    let cursor = Cursor::new(csv_data5);
    let dialect5 = sniffer.sniff(cursor)?;

    println!("Example 5 - Pipe-separated (using reader interface):");
    println!("  Delimiter: {:?}", dialect5.delimiter as char);
    println!("  Quote char: {:?}", dialect5.quote_char.map(|c| c as char));
    println!("  Has headers: {}", dialect5.has_headers);
    println!();

    // Example 6: Data without headers
    let csv_data6 = "John,25,NYC\nJane,30,LA\nBob,35,SF\nAlice,28,Chicago\nCharlie,32,Boston";
    let dialect6 = sniffer.sniff_from_string(csv_data6)?;

    println!("Example 6 - Data without headers:");
    println!("  Delimiter: {:?}", dialect6.delimiter as char);
    println!("  Quote char: {:?}", dialect6.quote_char.map(|c| c as char));
    println!("  Has headers: {}", dialect6.has_headers);
    println!();

    // Example 7: Mixed data types
    let csv_data7 = r#"id,name,email,age,salary,active,join_date
1,"John Doe",john@example.com,25,$50000.00,true,2023-01-15
2,"Jane Smith",jane@example.com,30,$65000.50,false,2022-06-20
3,"Bob Johnson",bob@example.com,35,$75000.25,true,2021-03-10"#;
    let dialect7 = sniffer.sniff_from_string(csv_data7)?;

    println!("Example 7 - Mixed data types:");
    println!("  Delimiter: {:?}", dialect7.delimiter as char);
    println!("  Quote char: {:?}", dialect7.quote_char.map(|c| c as char));
    println!("  Has headers: {}", dialect7.has_headers);

    Ok(())
}
