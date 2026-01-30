use shared::iostream::{FileStream, StreamError, IOStream};
use std::fs;
use std::path::Path;

#[test]
fn test_file_write_read() {
    let path = "test_io.txt";
    
    if Path::new(path).exists() {
        fs::remove_file(path).unwrap();
    }

    let mut stream = FileStream::new(path);
    
    assert!(stream.open_write(false).is_ok());
    assert!(stream.write("Hello, World!\n").is_ok());
    assert!(stream.write("Second Line").is_ok());
    stream.close();

    assert!(stream.open_read().is_ok());
    let content = stream.read_all();
    assert!(content.is_ok());
    assert_eq!(content.unwrap(), "Hello, World!\nSecond Line");
    stream.close();

    fs::remove_file(path).unwrap();
}

#[test]
fn test_file_append() {
    let path = "test_io_append.txt";
    if Path::new(path).exists() {
        fs::remove_file(path).unwrap();
    }

    let mut stream = FileStream::new(path);
    
    // Write initial
    stream.open_write(false).unwrap();
    stream.write("Line 1\n").unwrap();
    stream.close();

    // Append
    stream.open_write(true).unwrap();
    stream.write("Line 2").unwrap();
    stream.close();

    // Verify
    stream.open_read().unwrap();
    let content = stream.read_all().unwrap();
    assert_eq!(content, "Line 1\nLine 2");
    stream.close();

    fs::remove_file(path).unwrap();
}

#[test]
fn test_file_not_found() {
    let mut stream = FileStream::new("non_existent_file_xyz.txt");
    let result = stream.open_read();
    assert!(result.is_err());
    match result.unwrap_err() {
        StreamError::Io(ref e) if e.kind() == std::io::ErrorKind::NotFound => assert!(true),
        e => assert!(false, "Expected FileNotFound error, got {:?}", e),
    }
}

#[test]
fn test_console_stream_creation() {
    use std::io::BufReader;
    use std::io;
    let mut _stream = IOStream::new(BufReader::new(io::stdin()), io::stdout());
    // We just verify it compiles and initializes. 
    assert!(true);
}


#[test]
fn test_terminal_stream_read_line_and_write() {
    use std::io::Cursor;
    use std::io::BufReader;

    let input_bytes = b"Alpha\nBeta\n".to_vec();
    let reader = BufReader::new(Cursor::new(input_bytes));
    let writer: Vec<u8> = Vec::new();
    let mut stream = IOStream::new(reader, writer);

    // read a line
    let line = stream.read_line().unwrap();
    // Note: The new implementation trims the newline for convenience
    assert_eq!(line, "Alpha");

    // write and flush
    stream.write("Echoed\n").unwrap();
    stream.flush().unwrap();

    let out_vec = stream.into_writer();
    let out_str = String::from_utf8(out_vec).unwrap();
    assert_eq!(out_str, "Echoed\n");
}


#[test]
fn test_terminal_stream_read_all() {
    use std::io::Cursor;
    use std::io::BufReader;

    let input_bytes = b"All in one go".to_vec();
    let reader = BufReader::new(Cursor::new(input_bytes));
    let writer: Vec<u8> = Vec::new();
    let mut stream = IOStream::new(reader, writer);

    let all = stream.read_all().unwrap();
    assert_eq!(all, "All in one go");
}


#[test]
fn test_terminal_stream_echo_line() {
    use std::io::Cursor;
    use std::io::BufReader;

    let input_bytes = b"Echo me\n".to_vec();
    let reader = BufReader::new(Cursor::new(input_bytes));
    let writer: Vec<u8> = Vec::new();
    let mut stream = IOStream::new(reader, writer);

    let line = stream.read_line().unwrap();
    // Use writeln because read_line trims the newline
    stream.writeln(&line).unwrap();
    stream.flush().unwrap();

    let out_vec = stream.into_writer();
    let out_str = String::from_utf8(out_vec).unwrap();
    assert_eq!(out_str, "Echo me\n");
}
