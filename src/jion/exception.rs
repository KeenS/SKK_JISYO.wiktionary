use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exception {
    Override {
        candidate: String,
        modern: String,
        historical: String,
    },
    Excluded {
        candidate: String,
        modern: String,
    },
}

impl Exception {
    pub fn candidate(&self) -> &str {
        match self {
            Self::Override { candidate, .. } => candidate,
            Self::Excluded { candidate, .. } => candidate,
        }
    }

    pub fn modern(&self) -> &str {
        match self {
            Self::Override { modern, .. } => modern,
            Self::Excluded { modern, .. } => modern,
        }
    }
}

pub fn parse_line(line: &str) -> Option<Exception> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let mut fields = line.split('\t');
    let candidate = fields.next()?.trim();
    let modern = fields.next()?.trim();
    let historical = fields.next()?.trim();
    if fields.next().is_some() || candidate.is_empty() || modern.is_empty() {
        return None;
    }

    if historical == "-" {
        Some(Exception::Excluded {
            candidate: candidate.to_string(),
            modern: modern.to_string(),
        })
    } else if historical.is_empty() {
        None
    } else {
        Some(Exception::Override {
            candidate: candidate.to_string(),
            modern: modern.to_string(),
            historical: historical.to_string(),
        })
    }
}

pub fn read_exceptions(path: impl AsRef<Path>) -> io::Result<Vec<Exception>> {
    let file = File::open(path)?;
    let mut exceptions = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if let Some(exception) = parse_line(&line) {
            exceptions.push(exception);
        }
    }
    Ok(exceptions)
}

pub fn write_exceptions(path: impl AsRef<Path>, exceptions: &[Exception]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for exception in exceptions {
        match exception {
            Exception::Override {
                candidate,
                modern,
                historical,
            } => writeln!(writer, "{candidate}\t{modern}\t{historical}")?,
            Exception::Excluded { candidate, modern } => {
                writeln!(writer, "{candidate}\t{modern}\t-")?
            }
        }
    }
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_override_exception() {
        assert_eq!(
            parse_line("学校\tがっこう\tがっかう"),
            Some(Exception::Override {
                candidate: "学校".into(),
                modern: "がっこう".into(),
                historical: "がっかう".into(),
            })
        );
    }

    #[test]
    fn parses_excluded_exception() {
        assert_eq!(
            parse_line("学校\tがっこう\t-"),
            Some(Exception::Excluded {
                candidate: "学校".into(),
                modern: "がっこう".into(),
            })
        );
    }

    #[test]
    fn ignores_comments_and_invalid_lines() {
        assert_eq!(parse_line("# comment"), None);
        assert_eq!(parse_line("学校\tがっこう"), None);
        assert_eq!(parse_line("学校\tがっこう\tがっかう\textra"), None);
    }

    #[test]
    fn round_trips_exceptions() {
        let path = std::env::temp_dir().join("xml-xtract-exceptions-test");
        let exceptions = vec![
            Exception::Override {
                candidate: "学校".into(),
                modern: "がっこう".into(),
                historical: "がっかう".into(),
            },
            Exception::Excluded {
                candidate: "例外".into(),
                modern: "れいがい".into(),
            },
        ];
        write_exceptions(&path, &exceptions).unwrap();
        assert_eq!(read_exceptions(&path).unwrap(), exceptions);
        std::fs::remove_file(path).unwrap();
    }
}
