use anyhow::{Context, Result, bail};

/// Parser for messages in Portable Object format by GNU gettext
pub struct Parser {
  /// Number of plural cases in plural messages.
  pub number_of_plural_cases: Option<usize>,
}

#[derive(Debug,Clone,Hash,Eq,PartialEq,Ord,PartialOrd)]
pub enum PoMessage {
  Header {
    msgstr: String,
  },
  Regular {
    msgid: String,
    msgstr: String,
  },
  RegularWithContext {
    msgid: String,
    msgctxt: String,
    msgstr: String,
  },
  Plural {
    msgid: String,
    msgid_plural: String,
    msgstr: Vec<String>,
  },
  PluralWithContext {
    msgid: String,
    msgid_plural: String,
    msgctxt: String,
    msgstr: Vec<String>,
  },
}

impl PoMessage {
  pub fn to_key(&self) -> PoMessage {
    match self {
      Self::Header{..} => self.clone(),
      Self::Regular{msgid, ..} => Self::Regular{msgid: msgid.clone(), msgstr: "".to_string()},
      Self::RegularWithContext{msgctxt, msgid, ..} => Self::RegularWithContext{msgctxt: msgctxt.clone(), msgid: msgid.clone(), msgstr: "".to_string()},
      Self::Plural{msgid, msgid_plural, ..} => Self::Plural{msgid: msgid.clone(), msgid_plural: msgid_plural.clone(), msgstr: Vec::new()},
      Self::PluralWithContext{msgctxt, msgid, msgid_plural, ..} => Self::PluralWithContext{msgctxt: msgctxt.clone(), msgid:msgid.clone(), msgid_plural: msgid_plural.clone(), msgstr: Vec::new()},
    }
  }
}

pub fn escape_string(s: &String) -> String {
  let mut result = String::with_capacity(s.len());
  let mut prepend_quotes = false;

  let multiline = true; // TODO: make it global

  for (i, c) in s.chars().enumerate() {
    match c {
      '\r' => result.push_str("\\r"),
      '\n' if i+1 == s.len() => result.push_str("\\n"),
      '\n' if multiline => {
        prepend_quotes = true;
        result.push_str("\\n\"\n\"");
      },
      '\n' => result.push_str("\\n"),
      '\t' => result.push_str("\\t"),
      '"'  => result.push_str("\\\""),
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
    match self {
      Self::Header{msgstr} => {
        let msgstr = escape_string(msgstr);
        write!(f, "\
          msgid \"\"\n\
          msgstr \"{msgstr}\"\n\
        ")
      },
      Self::Regular{msgid, msgstr} => {
        let msgid = escape_string(msgid);
        let msgstr = escape_string(msgstr);
        write!(f, "\
          msgid \"{msgid}\"\n\
          msgstr \"{msgstr}\"\n\
        ")
      },

      Self::RegularWithContext{msgctxt, msgid, msgstr} => {
        let msgctxt = escape_string(msgctxt);
        let msgid = escape_string(msgid);
        let msgstr = escape_string(msgstr);
        write!(f, "\
          msgctxt \"{msgctxt}\"\n\
          msgid \"{msgid}\"\n\
          msgstr \"{msgstr}\"\n\
        ")
      },

      Self::Plural{msgid, msgid_plural, msgstr} => {
        let msgid = escape_string(msgid);
        let msgid_plural = escape_string(msgid_plural);
        write!(f, "\
          msgid \"{msgid}\"\n\
          msgid_plural \"{msgid_plural}\"\n\
        ")?;

        for (i, msgstr_i) in msgstr.iter().enumerate() {
          let msgstr_i = escape_string(msgstr_i);
          write!(f, "msgstr[{i}] \"{msgstr_i}\"\n")?;
        }

        Ok(())
      },

      Self::PluralWithContext{msgctxt, msgid, msgid_plural, msgstr} => {
        let msgctxt = escape_string(msgctxt);
        let msgid = escape_string(msgid);
        let msgid_plural = escape_string(msgid_plural);
        write!(f, "\
          msgctxt \"{msgctxt}\"\n\
          msgid \"{msgid}\"\n\
          msgid_plural \"{msgid_plural}\"\n\
        ")?;

        for (i, msgstr_i) in msgstr.iter().enumerate() {
          let msgstr_i = escape_string(msgstr_i);
          write!(f, "msgstr[{i}] \"{msgstr_i}\"\n")?;
        }

        Ok(())
      },
    }
  }
}



#[derive(Debug,Copy,Clone)]
enum Keyword {
  Msgctxt,
  Msgid,
  Msgstr,
  MsgidPlural,
  MsgstrPlural(u8),
}

impl std::fmt::Display for Keyword {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", match self {
      Self::Msgctxt => "msgctxt",
      Self::Msgid => "msgid",
      Self::Msgstr => "msgstr",
      Self::MsgidPlural => "msgid_plural",
      Self::MsgstrPlural(_n) => "msgstr[N]",
    })
  }
}

fn skip_spaces_and_comments(text: &[char]) -> &[char] {
  let mut tail = text;

  loop {
    match tail[..] {
      // Skip comment until end of line
      ['#', ..] | [ '/', '/', ..] => {
        loop {
          match tail[..] {
           ['\n', ..] | [] => break,
           _ => tail = &tail[1..],
          }
        }
      }

      // Skip whitespace
      [' ', ..] | ['\n', ..] => tail = &tail[1..],
      [c, ..] if c.is_whitespace() => tail = &tail[1..],
      _  => return tail,
    }
  }
}

fn skip_spaces(text: &[char]) -> &[char] {
  let mut tail = text;

  loop {
    match tail[..] {
      [' ', ..] | ['\n', ..] => tail = &tail[1..],
      [c, ..] if c.is_whitespace() => tail = &tail[1..],
      _ => return tail,
    }
  }
}

impl Parser {

  fn parse_keyword<'a>(&self, text: &'a[char]) -> Result<(Keyword, &'a[char])> {
    let tail = skip_spaces_and_comments(text);

    match tail[..] {
      ['m', 's', 'g', 'i', 'd', ' ', ..] => return Ok((Keyword::Msgid, &tail["msgid ".len()..])),
      ['m', 's', 'g', 's', 't', 'r', ' ', ..] => return Ok((Keyword::Msgstr, &tail["msgstr ".len()..])),
      ['m', 's', 'g', 's', 't', 'r', '[', num, ']', ' ',  ..] if num >='0' && num <= '9' => {
        return Ok((Keyword::MsgstrPlural(num.to_digit(10).unwrap() as u8) ,&tail["msgstr[0] ".len()..]));
      }
      ['m', 's', 'g', 'i', 'd', '_', 'p', 'l', 'u', 'r', 'a', 'l', ' ',  ..] => return Ok((Keyword::MsgidPlural, &tail["msgid_plural ".len()..])),
      ['m', 's', 'g', 'c', 't', 'x', 't', ' ', ..] => return Ok((Keyword::Msgctxt, &tail["msgctxt ".len()..])),
      [] => bail!("Unexpected end of text. Expected: msgid, msgstr, msgid_plural, msgstr[N]."),
      _ => bail!("Unexpected character or keyword. Expected: msgid, msgstr, msgid_plural, msgstr[N]. Text: \"{}\".", tail[..20.min(tail.len())].iter().collect::<String>()),
    }
  }

  fn parse_string<'a>(&self, text: &'a[char]) -> Result<(String, &'a[char])> {
    let mut s = String::new();
    let mut tail = skip_spaces(text);

    match tail[..] {
      // Starting quote
      ['"', ..] => tail = &tail[1..],

      [] => bail!("Unexpected end of text. Expected string sequence."),
      _ => bail!("Unexpected character at beginning of the string sequence. Expected: '\"'. Text: \"{}\".", tail[..20.min(tail.len())].iter().collect::<String>()),
    }

    loop {
      match tail[..] {
        // // String continues on next line
        ['"', '\n', '"'] => tail = &tail[2..],

        // Ending quote
        ['"', ..] => {
          tail = skip_spaces(&tail[1..]);
          match tail[..] {
            // String continues on next line
            ['"', ..] => {},

            // End of string
            _ => return Ok((s, tail)),
          }
       }

        // Escape sequence
        ['\\', c, ..] => {
          match c {
            'r' => s.push('\r'),
            'n' => s.push('\n'),
            't' => s.push('\t'),
            '"' => s.push('"'),
            '\\' => s.push('\\'),
            _ => bail!("Unexpected escape sequence in the string sequence. Expected: \\ followed by n, t, \", or \\. Text: \"{}\".", tail[..20.min(tail.len())].iter().collect::<String>()),
         }
         tail = &tail[1..];
        },

        // Raw control charactes in string
        ['\r', ..] => bail!("Unterminated string sequence. Expected: '\"' at the end of line."),
        ['\n', ..] => bail!("Unterminated string sequence. Expected: '\"' at the end of line."),
        ['\t', ..] => bail!("Raw tab character in the string sequence. Text: \"{}\".", tail[..20.min(tail.len())].iter().collect::<String>()),
        [c, ..] if c.is_control() => bail!("Raw control character in the string sequence. Text: \"{}\".", tail[..20.min(tail.len())].iter().collect::<String>()),

        // All other characters are added to string
        [c, ..] => s.push(c),

        [] => bail!("Unexpected end of text. Expected string sequence."),
      }

      tail = &tail[1..];
    }
  }

  pub fn parse_message(&self, text: &[char]) -> Result<PoMessage> {
    let mut msgctxt: Option<String> = None;
    let mut msgid: Option<String> = None;

    let mut tail = text;
    loop {
      let (kw, t) = self.parse_keyword(tail).context("Expected msgid \"...\" or msgctxt \"...\".")?;
      let (s, t) = self.parse_string(t).context("Expected msgid \"...\" or msgctxt \"...\".")?;
      tail = t;

      match kw {
        // Context
        Keyword::Msgctxt if msgctxt.is_none() && msgid.is_none() && !s.is_empty() => {
          msgctxt = Some(s);
          continue;
        },
        Keyword::Msgctxt if msgctxt.is_none() && msgid.is_none() && s.is_empty() => bail!("Empty context. Expected: non-empty msgctxt \"\"."),
        Keyword::Msgctxt if msgctxt.is_some() => bail!("Second msgctxt after first one. Expected: single msgctxt."),
        Keyword::Msgctxt if msgid.is_some() => bail!("msgctxt after msgid. Expected: msctxt before msgid."),


        // Header
        Keyword::Msgid if msgid.is_none() && s.len() == 0 => {
          let (kw, tail) = self.parse_keyword(tail).context("Expected msgstr \"...\" after empty msgid (AKA header).")?;
          let (s, tail) = self.parse_string(tail).context("Expected msgstr \"...\" after empty msgid (AKA header).")?;
          let tail = skip_spaces_and_comments(tail);

          match kw {
            // Header text
            Keyword::Msgstr if s.len() > 0 && tail.len() == 0 => {
              return Ok(PoMessage::Header { msgstr: s });
            },

            Keyword::Msgstr if s.len() == 0 && tail.len() == 0 => bail!("Expected non-empty string after msgstr in header. Actual string length: 0."),
            Keyword::Msgstr if s.len() > 0 && tail.len() > 0 => bail!("Garbage after msgstr in header Text: \"{}\".", tail[..20.min(tail.len())].iter().collect::<String>()),
            _ => bail!("Unexpected keyword after empty msgid (AKA header). Expected: msgstr. Actual keyword: {}.", kw),
          }
        },

        // Msgid
        Keyword::Msgid if msgid.is_none() => {
          msgid = Some(s);
          break;
        },

        // Something else instead of msgctxt or msgid
        _ => bail!("Unexpected keyword at beginning of the gettext PO message. Expected: msgid field with optional msgctxt before msgid. Actual keyword: {}.", kw),
      }

    }

    let (kw, tail) = self.parse_keyword(tail).context("Expected msgstr \"...\" or msgid_plural \"...\" after msgid.")?;
    let (s, tail) = self.parse_string(tail).context("Expected msgstr \"...\" or msgid_plural \"...\" after msgid.")?;

    match kw {
      // End of regular message
      Keyword::Msgstr => {
        let tail = skip_spaces_and_comments(tail);
        if !tail.is_empty() { bail!("Garbage after msgstr. Text: \"{}\".", tail[..20.min(tail.len())].iter().collect::<String>()); }

        match msgctxt {
          None => return Ok(PoMessage::Regular { msgid: msgid.unwrap(), msgstr: s }),
          Some(msgctxt) => return Ok(PoMessage::RegularWithContext { msgid: msgid.unwrap(), msgstr: s, msgctxt }),
        }
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

            Ok((Keyword::MsgstrPlural(n), _)) => bail!("Unexpected index of plural msgstr[N]. Expected index: {}, actual index: {}. Text: \"{}\".", msgstr.len(), n, tail[..20.min(text.len())].iter().collect::<String>()),
            Err(e) => return Err(e.context("Expected msgstr[N] \"...\" after msgid_plural \"...\" or msgstr[N] \"...\".")),
            Ok((kw,_)) => bail!("Unexpected keyword after msgid_plural. Expected: msgstr[N]. Actual keyword: {}.", kw),
          }
        }

        if let Some(number_of_plural_cases) = self.number_of_plural_cases {
          if msgstr.len() < number_of_plural_cases {
            for _ in 0..number_of_plural_cases-msgstr.len() {
              msgstr.push(String::new());
            }
          }
          msgstr.truncate(number_of_plural_cases);
        }

        let tail = skip_spaces_and_comments(tail);
        if tail.len() > 0 { bail!("Garbage after msgstr[N]. Text: \"{}\".", tail[..20.min(tail.len())].iter().collect::<String>()); }

        match msgctxt {
          None => return Ok(PoMessage::Plural { msgid: msgid.unwrap(), msgid_plural, msgstr}),
          Some(msgctxt) => return Ok(PoMessage::PluralWithContext { msgid: msgid.unwrap(), msgid_plural, msgstr, msgctxt }),
        }
      },

      kw => bail!("Unexpected keyword after msgid. Expected: msgid_plural, msgstr. Actual keyword: {}.", kw),
    }
  }

  pub fn parse_messages_from_stream(&self, stream: impl std::io::BufRead) -> Result<Vec<PoMessage>> {
    // Read lines from stdin, break at empty line, parse message
    let mut messages: Vec<PoMessage> = Vec::new();
    let mut buf = String::new();
    for (line_number, line) in stream.lines().enumerate() {
      let line = line?;
      let line = line.trim();

      if line.is_empty() && !buf.is_empty() {
        let buf_chars = buf.chars().collect::<Vec<char>>();

        let message = self.parse_message(&buf_chars[..]).context(format!("Cannot parse message at line #{line_number}. Message:\n\n{buf}"))?;
        messages.push(message);

        buf.truncate(0);
      } else {
        if !line.starts_with('#') {
          if !buf.is_empty() { buf += "\n"; }
          buf += line;
        }
      }
    }

    Ok(messages)
  }

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
    let chars: Vec<char> = orig.chars().chain("\n".chars()).collect();
    let parser = Parser { number_of_plural_cases: None };
    let msg = parser.parse_message(&chars[..]).expect("Message must be parsed correctly.");
    assert_eq!(orig, format!("{msg}"));
  }

  #[test]
  fn simple_message() {
    let orig = "\
msgid \"%d matching item\"
msgstr \"%d відповідний елемент\"
";
    let chars: Vec<char> = orig.chars().chain("\n".chars()).collect();
    let parser = Parser { number_of_plural_cases: None };
    let msg = parser.parse_message(&chars[..]).expect("Message must be parsed correctly.");
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

    let expected = r#"msgid ""
"\n"
"The minimum length for passwords consisting of characters from two classes\n"
"that don't meet requirements for passphrases: %s."
msgstr ""
"\n"
"Мінімальна довжина паролів, які складаються з символів двох класів\n"
"та не відповідають вимогам до парольних фраз: %s."
"#;
    let chars: Vec<char> = orig.chars().chain("\n".chars()).collect();
    let parser = Parser { number_of_plural_cases: None };
    let msg = parser.parse_message(&chars[..]).expect("Message must be parsed correctly.");
    assert_eq!(expected, format!("{msg}"));
  }

  #[test]
  fn simple_message_with_context() {
    let orig = "\
msgctxt \"listbox\"
msgid \"%d matching item\"
msgstr \"%d відповідний елемент\"
";
    let chars: Vec<char> = orig.chars().chain("\n".chars()).collect();
    let parser = Parser { number_of_plural_cases: None };
    let msg = parser.parse_message(&chars[..]).expect("Message must be parsed correctly.");
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
    let chars: Vec<char> = orig.chars().chain("\n".chars()).collect();
    let parser = Parser { number_of_plural_cases: None };
    let msg = parser.parse_message(&chars[..]).expect("Message must be parsed correctly.");
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
    let chars: Vec<char> = orig.chars().chain("\n".chars()).collect();
    let parser = Parser { number_of_plural_cases: None };
    let msg = parser.parse_message(&chars[..]).expect("Message must be parsed correctly.");
    assert_eq!(orig, format!("{msg}"));
  }

  #[test]
  fn simple_multiline_message() {
    let orig = "\
msgid \"foo\"
msgstr \"\"
\"bar\\n\"
\"baz\\n\"
";
    let chars: Vec<char> = orig.chars().chain("\n".chars()).collect();
    let parser = Parser { number_of_plural_cases: None };
    let msg = parser.parse_message(&chars[..]).expect("Message must be parsed correctly.");
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
msgid \"foo\"
msgstr \"\"
\"bar\\n\"
\"baz\\n\"
";
    let chars: Vec<char> = orig.chars().chain("\n".chars()).collect();
    let parser = Parser { number_of_plural_cases: None };
    let msg = parser.parse_message(&chars[..]).expect("Message must be parsed correctly.");
    assert_eq!(expected, format!("{msg}"));
  }

  #[test]
  fn no_message_error() {
    let orig = "\
# Foo
";
    let expected_err = "Unexpected end of text. Expected: msgid, msgstr, msgid_plural, msgstr[N].";

    let chars: Vec<char> = orig.chars().chain("\n".chars()).collect();
    let parser = Parser { number_of_plural_cases: None };
    let err = parser.parse_message(&chars[..]).unwrap_err();
    let err_root_cause = err.root_cause();
    assert_eq!(expected_err, format!("{err_root_cause}"));
  }

}
