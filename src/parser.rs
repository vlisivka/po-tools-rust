//! Parser and internal representation for GNU Gettext Portable Object (PO) files.
//!
//! This module provides the `Parser` struct for reading PO files and the `PoMessage`
//! struct to represent individual translation entries.

use anyhow::{Context, Result, bail};

/// Parser for messages in Portable Object format by GNU gettext.
pub struct Parser {
    /// Expected number of plural cases (e.g., from `nplurals=N` in the header).
    pub number_of_plural_cases: Option<usize>,
}

/// Represents a single message entry in a PO file.
///
/// A message can be a simple id-to-string translation, have a context,
/// or represent plural forms. It also handles the special "header" entry.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct PoMessage {
    /// Optional context (`msgctxt`).
    pub msgctxt: Option<String>,
    /// The original message string (`msgid`).
    pub msgid: String,
    /// The plural form of the original message (`msgid_plural`).
    pub msgid_plural: Option<String>,
    /// The translated strings (`msgstr`).
    /// - For regular messages: exactly one element.
    /// - For plural messages: N elements (one per plural form).
    /// - For headers: one element with header metadata.
    pub msgstr: Vec<String>,
}

impl PoMessage {
    /// Returns true if this is a header message (empty msgid).
    pub fn is_header(&self) -> bool {
        self.msgid.is_empty() && self.msgctxt.is_none()
    }

    /// Returns true if this is a plural message.
    pub fn is_plural(&self) -> bool {
        self.msgid_plural.is_some()
    }

    /// Returns true if this message has a context (msgctxt).
    pub fn has_context(&self) -> bool {
        self.msgctxt.is_some()
    }

    /// Returns true if this message is fully translated (all msgstr are non-empty).
    pub fn is_translated(&self) -> bool {
        !self.is_header() && self.msgstr.iter().all(|s| !s.is_empty())
    }

    /// Returns the first translated string (`msgstr[0]`), or an empty string if not present.
    pub fn msgstr_first(&self) -> &str {
        self.msgstr.first().map(|s| s.as_str()).unwrap_or("")
    }

    /// Creates a "key" version of the message by clearing its translations.
    /// This is useful for looking up messages in a map where only the identity matters.
    pub fn to_key(&self) -> Self {
        Self {
            msgctxt: self.msgctxt.clone(),
            msgid: self.msgid.clone(),
            msgid_plural: self.msgid_plural.clone(),
            msgstr: if self.is_header() {
                self.msgstr.clone()
            } else {
                Vec::new()
            },
        }
    }

    /// Combines this message's translation with another message's identity (key).
    pub fn with_key(&self, key: &Self) -> Self {
        Self {
            msgctxt: key.msgctxt.clone(),
            msgid: key.msgid.clone(),
            msgid_plural: key.msgid_plural.clone(),
            msgstr: self.msgstr.clone(),
        }
    }
}

/// Escapes a string for use in a PO file, handling newlines and quotes.
///
/// Supports multiline output strategy common in PO files.
pub fn escape_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prepend_quotes = false;

    let multiline = true; // TODO: make it global variable, to allow customization from command line
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

impl std::fmt::Display for PoMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Header
        if self.is_header() {
            let msgstr = escape_string(self.msgstr_first());
            return write!(
                f,
                "\
          msgid \"\"\n\
          msgstr \"{msgstr}\"\n\
        "
            );
        }

        // Optional msgctxt
        if let Some(ref msgctxt) = self.msgctxt {
            let msgctxt = escape_string(msgctxt);
            write!(f, "msgctxt \"{msgctxt}\"\n")?;
        }

        let msgid = escape_string(&self.msgid);

        // Plural message
        if let Some(ref msgid_plural) = self.msgid_plural {
            let msgid_plural = escape_string(msgid_plural);
            write!(
                f,
                "\
          msgid \"{msgid}\"\n\
          msgid_plural \"{msgid_plural}\"\n\
        "
            )?;

            for (i, msgstr_i) in self.msgstr.iter().enumerate() {
                let msgstr_i = escape_string(msgstr_i);
                writeln!(f, "msgstr[{i}] \"{msgstr_i}\"")?;
            }

            Ok(())
        } else {
            // Regular message
            let msgstr = escape_string(self.msgstr_first());
            write!(
                f,
                "\
          msgid  \"{msgid}\"\n\
          msgstr \"{msgstr}\"\n\
        "
            )
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Keyword {
    Msgctxt,
    Msgid,
    Msgstr,
    MsgidPlural,
    MsgstrPlural(u8),
}

impl std::fmt::Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Msgctxt => "msgctxt",
                Self::Msgid => "msgid",
                Self::Msgstr => "msgstr",
                Self::MsgidPlural => "msgid_plural",
                Self::MsgstrPlural(_n) => "msgstr[N]",
            }
        )
    }
}

fn skip_spaces_and_comments(text: &[u8]) -> &[u8] {
    let mut tail = text;

    loop {
        match tail {
            // Skip comment until end of line
            [b'#', ..] | [b'/', b'/', ..] => loop {
                match tail {
                    [b'\n', ..] | [] => break,
                    [_, rest @ ..] => tail = rest,
                }
            },

            // Skip whitespace
            [b' ', rest @ ..] | [b'\n', rest @ ..] | [b'\r', rest @ ..] | [b'\t', rest @ ..] => {
                tail = rest
            }
            _ => return tail,
        }
    }
}

fn skip_spaces(text: &[u8]) -> &[u8] {
    let mut tail = text;

    loop {
        match tail {
            [b' ', rest @ ..] | [b'\n', rest @ ..] | [b'\r', rest @ ..] | [b'\t', rest @ ..] => {
                tail = rest
            }
            _ => return tail,
        }
    }
}

fn snippet(tail: &[u8], max_len: usize) -> String {
    String::from_utf8_lossy(&tail[..max_len.min(tail.len())]).to_string()
}

impl Parser {
    /// Creates a new `Parser` instance.
    pub fn new(number_of_plural_cases: Option<usize>) -> Self {
        Self {
            number_of_plural_cases,
        }
    }

    #[rustfmt::skip]
    fn parse_keyword<'a>(&self, text: &'a [u8]) -> Result<(Keyword, &'a [u8])> {
        // TODO: Parse comments to support fuzzy messages
        let tail = skip_spaces_and_comments(text);

        match tail {
            [b'm', b's', b'g', b'i', b'd', b'_', b'p', b'l', b'u', b'r', b'a', b'l', b' ', rest @ ..] => Ok((Keyword::MsgidPlural, rest)),
            [b'm', b's', b'g', b'i', b'd', b' ', rest @ ..] => Ok((Keyword::Msgid, rest)),
            [b'm', b's', b'g', b's', b't', b'r', b'[', num, b']', b' ', rest @ ..] if num.is_ascii_digit() => {
                Ok((Keyword::MsgstrPlural(num - b'0'), rest))
            }
            [b'm', b's', b'g', b's', b't', b'r', b' ', rest @ ..] => Ok((Keyword::Msgstr, rest)),
            [b'm', b's', b'g', b'c', b't', b'x', b't', b' ', rest @ ..] => Ok((Keyword::Msgctxt, rest)),
            [] => {
                bail!("Unexpected end of text. Expected: msgid, msgstr, msgid_plural, msgstr[N].")
            }
            _ => bail!(
                "Unexpected character or keyword. Expected: msgid, msgstr, msgid_plural, msgstr[N]. Text: \"{}\".",
                snippet(tail, 20)
            ),
        }
    }

    fn parse_string<'a>(&self, text: &'a [u8]) -> Result<(String, &'a [u8])> {
        let mut buf = Vec::new();
        let mut tail = skip_spaces(text);

        match tail {
            // Starting quote
            [b'"', rest @ ..] => tail = rest,

            [] => bail!("Unexpected end of text. Expected string sequence."),
            _ => bail!(
                "Unexpected character at beginning of the string sequence. Expected: '\"'. Text: \"{}\".",
                snippet(tail, 20)
            ),
        }

        loop {
            match tail {
                // Ending quote of a segment
                [b'"', rest @ ..] => {
                    tail = skip_spaces(rest);
                    match tail {
                        // String continues on next line: consume the opening quote of the next segment
                        [b'"', rest @ ..] => {
                            tail = rest;
                            continue;
                        }

                        // End of the whole string sequence
                        _ => {
                            let s = String::from_utf8(buf)
                                .context("Invalid UTF-8 in PO message string")?;
                            return Ok((s, tail));
                        }
                    }
                }

                // Escape sequence
                [b'\\', c, rest @ ..] => {
                    match c {
                        b'r' => buf.push(b'\r'),
                        b'n' => buf.push(b'\n'),
                        b't' => buf.push(b'\t'),
                        b'"' => buf.push(b'"'),
                        b'\\' => buf.push(b'\\'),
                        _ => bail!(
                            "Unexpected escape sequence in the string sequence. Expected: \\ followed by n, t, \", or \\. Text: \"{}\".",
                            snippet(tail, 20)
                        ),
                    }
                    tail = rest;
                    continue;
                }

                // Raw control characters in string
                [b'\r', ..] => {
                    bail!("Unterminated string sequence. Expected: '\"' at the end of line.")
                }
                [b'\n', ..] => {
                    bail!("Unterminated string sequence. Expected: '\"' at the end of line.")
                }
                [b'\t', ..] => bail!(
                    "Raw tab character in the string sequence. Text: \"{}\".",
                    snippet(tail, 20)
                ),
                [c, rest @ ..] if c.is_ascii_control() => bail!(
                    "Raw control character in the string sequence. Text: \"{}\".",
                    snippet(tail, 20)
                ),

                // All other bytes are added to buffer
                [c, rest @ ..] => {
                    buf.push(*c);
                    tail = rest;
                    continue;
                }

                [] => bail!("Unexpected end of text. Expected string sequence."),
            }
        }
    }

    /// Parses a single message entry from a string.
    pub fn parse_message_from_str(&self, text: &str) -> Result<PoMessage> {
        self.parse_message(text.as_bytes())
    }

    /// Parses a single message entry from a byte slice.
    pub fn parse_message(&self, text: &[u8]) -> Result<PoMessage> {
        let mut msgctxt: Option<String> = None;
        let mut msgid: Option<String> = None;

        let mut tail = text;
        loop {
            // TODO: Parse comments to support fuzzy messages
            let (kw, t) = self
                .parse_keyword(tail)
                .context("Expected msgid \"...\" or msgctxt \"...\".")?;
            let (s, t) = self
                .parse_string(t)
                .context("Expected msgid \"...\" or msgctxt \"...\".")?;
            tail = t;

            match kw {
                // Context
                Keyword::Msgctxt if msgctxt.is_none() && msgid.is_none() && !s.is_empty() => {
                    msgctxt = Some(s);
                    continue;
                }
                Keyword::Msgctxt if msgctxt.is_none() && msgid.is_none() && s.is_empty() => {
                    bail!("Empty context. Expected: non-empty msgctxt \"\".")
                }
                Keyword::Msgctxt if msgctxt.is_some() => {
                    bail!("Second msgctxt after first one. Expected: single msgctxt.")
                }
                Keyword::Msgctxt if msgid.is_some() => {
                    bail!("msgctxt after msgid. Expected: msctxt before msgid.")
                }

                // Header
                Keyword::Msgid if msgid.is_none() && s.is_empty() => {
                    let (kw, tail) = self
                        .parse_keyword(tail)
                        .context("Expected msgstr \"...\" after empty msgid (AKA header).")?;
                    let (s, tail) = self
                        .parse_string(tail)
                        .context("Expected msgstr \"...\" after empty msgid (AKA header).")?;
                    let tail = skip_spaces_and_comments(tail);

                    match kw {
                        // Header text
                        Keyword::Msgstr if !s.is_empty() && tail.is_empty() => {
                            return Ok(PoMessage {
                                msgctxt: None,
                                msgid: String::new(),
                                msgid_plural: None,
                                msgstr: vec![s],
                            });
                        }

                        Keyword::Msgstr if s.is_empty() && tail.is_empty() => bail!(
                            "Expected non-empty string after msgstr in header. Actual string length: 0."
                        ),
                        Keyword::Msgstr if !s.is_empty() && !tail.is_empty() => bail!(
                            "Garbage after msgstr in header Text: \"{}\".",
                            snippet(tail, 20)
                        ),
                        _ => bail!(
                            "Unexpected keyword after empty msgid (AKA header). Expected: msgstr. Actual keyword: {}.",
                            kw
                        ),
                    }
                }

                // Msgid
                Keyword::Msgid if msgid.is_none() => {
                    msgid = Some(s);
                    break;
                }

                // Something else instead of msgctxt or msgid
                _ => bail!(
                    "Unexpected keyword at beginning of the gettext PO message. Expected: msgid field with optional msgctxt before msgid. Actual keyword: {}.",
                    kw
                ),
            }
        }

        let (kw, tail) = self
            .parse_keyword(tail)
            .context("Expected msgstr \"...\" or msgid_plural \"...\" after msgid.")?;
        let (s, tail) = self
            .parse_string(tail)
            .context("Expected msgstr \"...\" or msgid_plural \"...\" after msgid.")?;

        match kw {
            // End of regular message
            Keyword::Msgstr => {
                let _tail = skip_spaces_and_comments(tail);
                // FIXME: add option to ignore garbage after end of msgstr:
                //if !tail.is_empty() { bail!("Garbage after msgstr. Text: \"{}\".", snippet(tail, 20)); }

                Ok(PoMessage {
                    msgctxt,
                    msgid: msgid.unwrap(),
                    msgid_plural: None,
                    msgstr: vec![s],
                })
            }

            // Plural message
            Keyword::MsgidPlural => {
                let msgid_plural = s;
                let mut msgstr: Vec<String> = Vec::new();

                let mut tail = tail;
                while !tail.is_empty() {
                    match self.parse_keyword(tail) {
            // Plural msgstr[N]
            Ok((Keyword::MsgstrPlural(n), t)) if msgstr.len() == n as usize => {
              let (s, t) = self.parse_string(t)?;
              msgstr.push(s);
              tail = t;
            },

            Ok((Keyword::MsgstrPlural(n), _)) => bail!("Unexpected index of plural msgstr[N]. Expected index: {}, actual index: {}. Text: \"{}\".", msgstr.len(), n, snippet(tail, 20)),
            Err(e) => return Err(e.context("Expected msgstr[N] \"...\" after msgid_plural \"...\" or msgstr[N] \"...\".")),
            Ok((kw,_)) => bail!("Unexpected keyword after msgid_plural. Expected: msgstr[N]. Actual keyword: {}.", kw),
          }
                }

                if let Some(number_of_plural_cases) = self.number_of_plural_cases {
                    if msgstr.len() < number_of_plural_cases {
                        for _ in 0..number_of_plural_cases - msgstr.len() {
                            msgstr.push(String::new());
                        }
                    }
                    msgstr.truncate(number_of_plural_cases);
                }

                let tail = skip_spaces_and_comments(tail);
                if !tail.is_empty() {
                    bail!("Garbage after msgstr[N]. Text: \"{}\".", snippet(tail, 20));
                }

                Ok(PoMessage {
                    msgctxt,
                    msgid: msgid.unwrap(),
                    msgid_plural: Some(msgid_plural),
                    msgstr,
                })
            }

            kw => bail!(
                "Unexpected keyword after msgid. Expected: msgid_plural, msgstr. Actual keyword: {}.",
                kw
            ),
        }
    }

    /// Parses multiple messages from a stream that implements `BufRead`.
    pub fn parse_messages_from_stream(
        &self,
        stream: impl std::io::BufRead,
    ) -> Result<Vec<PoMessage>> {
        // Read lines from stdin, break at empty line, parse message
        let mut messages: Vec<PoMessage> = Vec::new();
        let mut buf = String::new();
        for (line_number, line) in stream.lines().enumerate() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() && !buf.is_empty() {
                let message = self.parse_message_from_str(&buf).context(format!(
                    "Cannot parse message at line #{line_number}. Message:\n\n{buf}"
                ))?;
                messages.push(message);

                buf.truncate(0);
            } else if !line.starts_with('#') {
                if !buf.is_empty() {
                    buf += "\n";
                }
                buf += line;
            }
        }
        if !buf.is_empty() {
            let message = self.parse_message_from_str(&buf).context(format!(
                "Cannot parse message at end of stream. Message:\n\n{buf}"
            ))?;
            messages.push(message);
        }

        Ok(messages)
    }

    /// Parses multiple messages from a string.
    pub fn parse_messages_from_str(&self, s: &str) -> Result<Vec<PoMessage>> {
        self.parse_messages_from_stream(s.as_bytes())
    }

    /// Parses multiple messages from a file. If path is "-", reads from stdin.
    pub fn parse_messages_from_file(&self, file: &str) -> Result<Vec<PoMessage>> {
        if file == "-" {
            self.parse_messages_from_stream(std::io::stdin().lock())
        } else {
            let f = std::fs::File::open(file)?;
            let f = std::io::BufReader::new(f);

            self.parse_messages_from_stream(f)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header() {
        let orig = "\
msgid \"\"
msgstr \"\"
\"Key: value\\n\"
\"Key2: value2\\n\"
\"Key3: value3\\n\"
";
        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let msg = parser
            .parse_message(&bytes[..])
            .expect("Message must be parsed correctly.");
        assert_eq!(orig, format!("{msg}"));
    }

    #[test]
    fn simple_message() {
        let orig = "\
msgid  \"%d matching item\"
msgstr \"%d відповідний елемент\"
";
        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let msg = parser
            .parse_message(&bytes[..])
            .expect("Message must be parsed correctly.");
        assert_eq!(orig, format!("{msg}"));
    }

    #[test]
    fn simple_message_with_whitespace() {
        let orig = r#"

    msgid ""
    "\n"
    "The minimum length for passwords consisting of characters from two classes\n"
    "that don't meet requirements for passphrases: %s."
    msgstr ""
    "\n"
    "Мінімальна довжина паролів, які складаються з символів двох класів\n"
    "та не відповідають вимогам до парольних фраз: %s."
"#;

        let expected = r#"msgid  ""
"\n"
"The minimum length for passwords consisting of characters from two classes\n"
"that don't meet requirements for passphrases: %s."
msgstr ""
"\n"
"Мінімальна довжина паролів, які складаються з символів двох класів\n"
"та не відповідають вимогам до парольних фраз: %s."
"#;
        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let msg = parser
            .parse_message(&bytes[..])
            .expect("Message must be parsed correctly.");
        assert_eq!(expected, format!("{msg}"));
    }

    #[test]
    fn simple_message_with_context() {
        let orig = "\
msgctxt \"listbox\"
msgid  \"%d matching item\"
msgstr \"%d відповідний елемент\"
";
        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let msg = parser
            .parse_message(&bytes[..])
            .expect("Message must be parsed correctly.");
        assert_eq!(orig, format!("{msg}"));
    }

    #[test]
    fn plural_message() {
        let orig = "\
msgid \"%d matching item\"
msgid_plural \"%d matching items\"
msgstr[0] \"%d відповідний елемент\"
msgstr[1] \"%d відповідні елементи\"
msgstr[2] \"%d відповідних елементів\"
";
        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let msg = parser
            .parse_message(&bytes[..])
            .expect("Message must be parsed correctly.");
        assert_eq!(orig, format!("{msg}"));
    }

    #[test]
    fn plural_message_with_context() {
        let orig = "\
msgctxt \"listbox\"
msgid \"%d matching item\"
msgid_plural \"%d matching items\"
msgstr[0] \"%d відповідний елемент\"
msgstr[1] \"%d відповідні елементи\"
msgstr[2] \"%d відповідних елементів\"
";
        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let msg = parser
            .parse_message(&bytes[..])
            .expect("Message must be parsed correctly.");
        assert_eq!(orig, format!("{msg}"));
    }

    #[test]
    fn simple_multiline_message() {
        let orig = "\
msgid  \"foo\"
msgstr \"\"
\"bar\\n\"
\"baz\\n\"
";
        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let msg = parser
            .parse_message(&bytes[..])
            .expect("Message must be parsed correctly.");
        assert_eq!(orig, format!("{msg}"));
    }

    #[test]
    fn simple_singleline_message_with_endline() {
        let orig = r#"msgid  "Only one of -s, -g, -r, or -l allowed\n"
msgstr "Дозволено лише одне з -s, -g, -r або -l\n"
"#;
        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let msg = parser
            .parse_message(&bytes[..])
            .expect("Message must be parsed correctly.");
        assert_eq!(orig, format!("{msg}"));
    }

    #[test]
    fn simple_message_with_comments() {
        let orig = "\
# Foo
msgid \"foo\"
# Bar
msgstr \"\"
\"bar\\n\"
\"baz\\n\"
# Baz
";
        let expected = "\
msgid  \"foo\"
msgstr \"\"
\"bar\\n\"
\"baz\\n\"
";
        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let msg = parser
            .parse_message(&bytes[..])
            .expect("Message must be parsed correctly.");
        assert_eq!(expected, format!("{msg}"));
    }

    #[test]
    fn no_message_error() {
        let orig = "\
# Foo
";
        let expected_err =
            "Unexpected end of text. Expected: msgid, msgstr, msgid_plural, msgstr[N].";

        let bytes: Vec<u8> = orig.bytes().chain(b"\n".iter().copied()).collect();
        let parser = Parser {
            number_of_plural_cases: None,
        };
        let err = parser.parse_message(&bytes[..]).unwrap_err();
        let err_root_cause = err.root_cause();
        assert_eq!(expected_err, format!("{err_root_cause}"));
    }
}
