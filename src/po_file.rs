//! PO file reading and writing.
//!
//! `PoFileReader` handles parsing PO files with BOM detection.
//! `PoFileWriter` handles serializing PoMessage to PO format with configurable escaping.

use crate::parser::{Parser, PoMessage};
use anyhow::Result;
use std::io::{Read, Seek, Write};
use unicode_bom::Bom;

// ---------------------------------------------------------------------------
// PoFileReader — reading PO files
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct PoFileReader;

#[allow(dead_code)]
impl PoFileReader {
    /// Parses messages from a file or stdin ("-"), handling BOM detection.
    pub fn parse_messages_from_file(parser: &Parser, path: &str) -> Result<Vec<PoMessage>> {
        if path == "-" {
            return Self::parse_messages_from_stdin(parser);
        }
        let f = std::fs::File::open(path)?;
        let f = std::io::BufReader::new(f);
        Self::parse_messages_from_read(parser, f)
    }

    /// Parses messages from stdin with BOM detection.
    fn parse_messages_from_stdin(parser: &Parser) -> Result<Vec<PoMessage>> {
        let mut raw = Vec::new();
        std::io::stdin().lock().read_to_end(&mut raw)?;
        Self::parse_messages_from_read(parser, std::io::Cursor::new(raw))
    }

    /// Parses messages from a seekable read stream with BOM detection.
    pub fn parse_messages_from_read(
        parser: &Parser,
        mut reader: impl Read + Seek,
    ) -> Result<Vec<PoMessage>> {
        let mut buf = [0u8; 4];
        let bytes_read = reader.read(&mut buf)?;
        let bom = Bom::from(&buf[0..bytes_read]);

        match bom {
            Bom::Null | Bom::Utf8 => {}
            _ => return Err(anyhow::anyhow!("Unsupported BOM: {bom}")),
        }

        reader.seek(std::io::SeekFrom::Start(bom.len() as u64))?;
        let mut wrapped = std::io::BufReader::new(reader);
        parser.parse_messages_from_stream(&mut wrapped)
    }
}

// ---------------------------------------------------------------------------
// PoFileWriter — writing PO files
// ---------------------------------------------------------------------------

pub struct PoFileConfig {
    pub multiline: bool,
}

pub struct PoFileWriter {
    config: PoFileConfig,
}

impl Default for PoFileWriter {
    fn default() -> Self {
        // multiline=true matches the current hardcoded behavior in parser.rs
        Self::new(PoFileConfig { multiline: true })
    }
}

impl PoFileWriter {
    pub fn new(config: PoFileConfig) -> Self {
        Self { config }
    }

    /// Serializes a single message to a String.
    #[allow(dead_code)]
    pub fn write_message_as_string(&self, message: &PoMessage) -> String {
        let mut buf = Vec::new();
        self.write_message(message, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    /// Writes a single message to the writer.
    pub fn write_message(&self, message: &PoMessage, writer: &mut dyn Write) -> Result<()> {
        // Comments
        for comment in &message.comments {
            writeln!(writer, "{}", comment)?;
        }

        // Nothing (comment-only block)
        if message.is_nothing() {
            return Ok(());
        }

        // Header
        if message.is_header() {
            let msgstr = Self::escape_string(self.config.multiline, message.msgstr_single());
            write!(writer, "msgid \"\"\nmsgstr \"{msgstr}\"\n")?;
            return Ok(());
        }

        // Optional msgctxt
        if let Some(ref msgctxt) = message.msgctxt {
            let msgctxt = Self::escape_string(self.config.multiline, msgctxt);
            writeln!(writer, "msgctxt \"{msgctxt}\"")?;
        }

        let msgid = Self::escape_string(self.config.multiline, &message.msgid);

        // Plural message
        if let Some(ref msgid_plural) = message.msgid_plural {
            let msgid_plural = Self::escape_string(self.config.multiline, msgid_plural);
            write!(
                writer,
                "\
          msgid \"{msgid}\"\n\
          msgid_plural \"{msgid_plural}\"\n\
        "
            )?;

            for (i, msgstr_i) in message.msgstr.iter().enumerate() {
                let msgstr_i = Self::escape_string(self.config.multiline, msgstr_i);
                writeln!(writer, "msgstr[{i}] \"{msgstr_i}\"")?;
            }

            Ok(())
        } else {
            // Regular message
            let msgstr = Self::escape_string(self.config.multiline, message.msgstr_single());
            write!(
                writer,
                "\
          msgid \"{msgid}\"\n\
          msgstr \"{msgstr}\"\n\
        "
            )?;
            Ok(())
        }
    }

    #[allow(dead_code)]
    pub fn write(&self, messages: &[PoMessage], writer: &mut dyn Write) -> Result<()> {
        for message in messages {
            self.write_message(message, writer)?;
        }
        Ok(())
    }

    /// Private: escapes a string for PO output.
    fn escape_string(multiline: bool, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut prepend_quotes = false;

        let len = s.chars().count();

        for (i, c) in s.chars().enumerate() {
            match c {
                '\r' => result.push_str("\\r"),

                // If newline character is last character in the string, then don't make string multiline.
                '\n' if i + 1 == len => result.push_str("\\n"),

                // If string contains newline character, then make it multiline, when requested
                '\n' if multiline => {
                    prepend_quotes = true;
                    result.push_str("\\n\"\n\"");
                }

                '\n' => result.push_str("\\n"),
                '\t' => result.push_str("\\t"),
                '"' => result.push_str("\\\""),
                '\\' => result.push_str("\\\\"),
                _ => result.push(c),
            }
        }

        if prepend_quotes {
            result.insert_str(0, "\"\n\"");
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_message_escapes_newline_when_multiline_false() {
        let mut out = Vec::new();
        let writer = PoFileWriter::new(PoFileConfig { multiline: false });

        let message = PoMessage {
            msgid: "hello\nworld".to_string(),
            msgstr: vec!["привіт\nсвіт".to_string()],
            ..Default::default()
        };

        writer.write_message(&message, &mut out).unwrap();

        let result = String::from_utf8(out).unwrap();
        // Newline should be escaped as literal \n when multiline is false
        assert!(result.contains("hello\\nworld"));
    }

    #[test]
    fn write_message_uses_multiline_format_when_configured() {
        let mut out = Vec::new();
        let writer = PoFileWriter::new(PoFileConfig { multiline: true });

        let message = PoMessage {
            msgid: "hello\nworld".to_string(),
            msgstr: vec!["привіт\nсвіт".to_string()],
            ..Default::default()
        };

        writer.write_message(&message, &mut out).unwrap();

        let result = String::from_utf8(out).unwrap();
        // Newline should use multiline format when multiline is true
        assert!(result.contains("\\n\"\n\""));
    }
}
