// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug)]
pub enum StreamError {
    Io(io::Error),
    Format(fmt::Error),
    Parse(String),
    NotOpen,
    ModeMismatch,
    Encoding(String),
}

impl StreamError {
    pub fn msg(&self) -> String {
        match self {
            StreamError::Io(e) => format!("[IO] {}", e),
            StreamError::Format(e) => format!("[Format] {}", e),
            StreamError::Parse(e) => format!("[Parse] {}", e),
            StreamError::NotOpen => "[IO] Поток не открыт".to_string(),
            StreamError::ModeMismatch => "[IO] Неверный режим доступа".to_string(),
            StreamError::Encoding(e) => format!("[Encoding] {}", e),
        }
    }
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg())
    }
}

impl From<io::Error> for StreamError {
    fn from(e: io::Error) -> Self {
        StreamError::Io(e)
    }
}

impl From<fmt::Error> for StreamError {
    fn from(e: fmt::Error) -> Self {
        StreamError::Format(e)
    }
}

// ============================================================================
// Core IO Stream
// ============================================================================

/// Универсальная структура потока ввода-вывода.
/// Поддерживает буферизацию, форматированный вывод и парсинг ввода.
pub struct IOStream<R, W> {
    reader: R,
    writer: W,
    buffer: String,
}

impl<R: BufRead, W: Write> IOStream<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            buffer: String::new(),
        }
    }

    // --- Output Operations ---

    pub fn write(&mut self, data: &str) -> Result<(), StreamError> {
        self.writer.write_all(data.as_bytes())?;
        Ok(())
    }

    pub fn writeln(&mut self, data: &str) -> Result<(), StreamError> {
        self.writer.write_all(data.as_bytes())?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }

    pub fn write_fmt(&mut self, args: fmt::Arguments) -> Result<(), StreamError> {
        self.writer.write_fmt(args)?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), StreamError> {
        self.writer.flush()?;
        Ok(())
    }

    // --- Input Operations ---

    pub fn read_line(&mut self) -> Result<String, StreamError> {
        self.buffer.clear();
        let bytes = self.reader.read_line(&mut self.buffer)?;
        if bytes == 0 {
            return Err(StreamError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "EOF",
            )));
        }
        Ok(self.buffer.trim_end().to_string())
    }

    pub fn read_all(&mut self) -> Result<String, StreamError> {
        let mut content = String::new();
        self.reader.read_to_string(&mut content)?;
        Ok(content)
    }

    /// Читает следующее слово (токен), пропуская пробелы.
    pub fn read_token(&mut self) -> Result<String, StreamError> {
        let mut token = String::new();
        let mut char_buf = [0u8; 1];
        let mut started = false;

        loop {
            match self.reader.read(&mut char_buf) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let c = char_buf[0] as char;
                    if c.is_whitespace() {
                        if started {
                            break;
                        } // End of token
                    } else {
                        started = true;
                        token.push(c);
                    }
                }
                Err(e) => return Err(StreamError::Io(e)),
            }
        }

        if token.is_empty() && !started {
            return Err(StreamError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "EOF",
            )));
        }
        Ok(token)
    }

    /// Парсит следующий токен в указанный тип.
    pub fn parse<T: std::str::FromStr>(&mut self) -> Result<T, StreamError> {
        let token = self.read_token()?;
        token
            .parse::<T>()
            .map_err(|_| StreamError::Parse(format!("Failed to parse '{}'", token)))
    }

    pub fn into_writer(self) -> W {
        self.writer
    }
}

// ============================================================================
// File Stream
// ============================================================================

enum FileMode {
    Closed,
    Read(BufReader<File>),
    Write(BufWriter<File>),
}

pub struct FileStream {
    path: String,
    mode: FileMode,
}

impl FileStream {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            mode: FileMode::Closed,
        }
    }

    pub fn open_read(&mut self) -> Result<(), StreamError> {
        let file = File::open(&self.path).map_err(StreamError::Io)?;
        self.mode = FileMode::Read(BufReader::new(file));
        Ok(())
    }

    pub fn open_write(&mut self, append: bool) -> Result<(), StreamError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(append)
            .truncate(!append)
            .open(&self.path)?;
        self.mode = FileMode::Write(BufWriter::new(file));
        Ok(())
    }

    pub fn close(&mut self) {
        if let FileMode::Write(ref mut w) = self.mode {
            let _ = w.flush();
        }
        self.mode = FileMode::Closed;
    }

    pub fn read_line(&mut self) -> Result<String, StreamError> {
        match &mut self.mode {
            FileMode::Read(reader) => {
                let mut buf = String::new();
                reader.read_line(&mut buf)?;
                Ok(buf)
            }
            _ => Err(StreamError::ModeMismatch),
        }
    }

    pub fn read_all(&mut self) -> Result<String, StreamError> {
        match &mut self.mode {
            FileMode::Read(reader) => {
                let mut buf = String::new();
                reader.read_to_string(&mut buf)?;
                Ok(buf)
            }
            _ => Err(StreamError::ModeMismatch),
        }
    }

    pub fn write(&mut self, data: &str) -> Result<(), StreamError> {
        match &mut self.mode {
            FileMode::Write(writer) => {
                writer.write_all(data.as_bytes())?;
                Ok(())
            }
            _ => Err(StreamError::ModeMismatch),
        }
    }

    pub fn flush(&mut self) -> Result<(), StreamError> {
        match &mut self.mode {
            FileMode::Write(writer) => {
                writer.flush()?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

impl Drop for FileStream {
    fn drop(&mut self) {
        self.close();
    }
}
