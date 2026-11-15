//! Minimal parser for Valve's text key-value format.

use thiserror::Error;

/// A parsed VDF value: a string or a table of key-value pairs.
#[derive(Debug, PartialEq)]
pub enum Value {
    Str(String),
    Map(Vec<(String, Value)>),
}

impl Value {
    /// Returns the value stored under the given key.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.as_map()?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, value)| value)
    }

    /// Returns the string this value holds, if not a table.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Map(_) => None,
        }
    }

    /// Returns the pairs this value holds, if not a string.
    pub fn as_map(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Map(pairs) => Some(pairs),
            Value::Str(_) => None,
        }
    }
}

/// A VDF parse failure, located by line number.
#[derive(Debug, Error)]
#[error("line {line}: {message}")]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

/// Parses a whole document into its top-level key-value pairs.
pub fn parse(input: &str) -> Result<Value, ParseError> {
    let mut lexer = Lexer::new(input);
    let pairs = parse_pairs(&mut lexer, false)?;
    if lexer.next()?.is_some() {
        return Err(lexer.error("unexpected input after the top level"));
    }
    Ok(Value::Map(pairs))
}

/// Parses key-value pairs until the input ends, or a closing brace when nested.
fn parse_pairs(lexer: &mut Lexer, nested: bool) -> Result<Vec<(String, Value)>, ParseError> {
    let mut pairs = Vec::new();
    loop {
        match lexer.next()? {
            None => {
                if nested {
                    return Err(lexer.error("unclosed brace"));
                }
                return Ok(pairs);
            }
            Some(Token::Close) if nested => return Ok(pairs),
            Some(Token::Close) => return Err(lexer.error("unexpected closing brace")),
            Some(Token::Open) => return Err(lexer.error("expected a key, found `{`")),
            Some(Token::Str(key)) => {
                let value = match lexer.next()? {
                    Some(Token::Str(s)) => Value::Str(s),
                    Some(Token::Open) => Value::Map(parse_pairs(lexer, true)?),
                    _ => return Err(lexer.error("expected a value")),
                };
                pairs.push((key, value));
            }
        }
    }
}

enum Token {
    Str(String),
    Open,
    Close,
}

struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input: input.as_bytes(),
            pos: 0,
            line: 1,
        }
    }

    fn next(&mut self) -> Result<Option<Token>, ParseError> {
        self.skip_trivia();
        match self.input.get(self.pos) {
            None => Ok(None),
            Some(b'{') => {
                self.pos += 1;
                Ok(Some(Token::Open))
            }
            Some(b'}') => {
                self.pos += 1;
                Ok(Some(Token::Close))
            }
            Some(b'"') => {
                self.pos += 1;
                Ok(Some(Token::Str(self.quoted_string()?)))
            }
            Some(_) => Ok(Some(Token::Str(self.bare_token()))),
        }
    }

    /// Consumes whitespace and `//` comments.
    fn skip_trivia(&mut self) {
        while let Some(&b) = self.input.get(self.pos) {
            match b {
                b'\n' => {
                    self.line += 1;
                    self.pos += 1;
                }
                b' ' | b'\t' | b'\r' => self.pos += 1,
                b'/' if self.input.get(self.pos + 1) == Some(&b'/') => {
                    while let Some(&b) = self.input.get(self.pos) {
                        self.pos += 1;
                        if b == b'\n' {
                            self.line += 1;
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    /// Reads a quoted string, resolving escapes and line numbers as it goes.
    fn quoted_string(&mut self) -> Result<String, ParseError> {
        let start_line = self.line;
        let mut out = String::new();
        let mut segment = self.pos;
        loop {
            let Some(&b) = self.input.get(self.pos) else {
                return Err(self.error_at(start_line, "unterminated string"));
            };
            match b {
                b'"' => {
                    out.push_str(&self.lossy(segment, self.pos));
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    out.push_str(&self.lossy(segment, self.pos));
                    self.pos += 1;
                    let Some(&escaped) = self.input.get(self.pos) else {
                        return Err(self.error_at(start_line, "unterminated string"));
                    };
                    match escaped {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        other => out.push(other as char),
                    }
                    self.pos += 1;
                    segment = self.pos;
                }
                b'\n' => {
                    self.line += 1;
                    self.pos += 1;
                }
                _ => self.pos += 1,
            }
        }
    }

    /// Reads an unquoted token, ending at whitespace, a quote, or a brace.
    fn bare_token(&mut self) -> String {
        let start = self.pos;
        while let Some(&b) = self.input.get(self.pos) {
            match b {
                b' ' | b'\t' | b'\r' | b'\n' | b'"' | b'{' | b'}' => break,
                _ => self.pos += 1,
            }
        }
        self.lossy(start, self.pos)
    }

    /// Decodes a byte range, tolerating invalid UTF-8 the way Steam writes it.
    fn lossy(&self, start: usize, end: usize) -> String {
        String::from_utf8_lossy(&self.input[start..end]).into_owned()
    }

    fn error(&self, message: &str) -> ParseError {
        self.error_at(self.line, message)
    }

    fn error_at(&self, line: usize, message: &str) -> ParseError {
        ParseError {
            line,
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_pairs_and_nested_tables() {
        let input = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"apps"
		{
			"730"		"74265067316"
		}
	}
}
"#;
        let value = parse(input).unwrap();
        let folder = value.get("libraryfolders").unwrap().get("0").unwrap();
        assert_eq!(
            folder.get("path").unwrap().as_str(),
            Some(r"C:\Program Files (x86)\Steam")
        );
        assert_eq!(
            folder.get("apps").unwrap().get("730").unwrap().as_str(),
            Some("74265067316")
        );
    }

    #[test]
    fn parses_unquoted_tokens_and_ignores_comments() {
        let value = parse("key // comment\n{\nsubkey value\n}").unwrap();
        assert_eq!(
            value.get("key").unwrap().get("subkey").unwrap().as_str(),
            Some("value")
        );
    }

    #[test]
    fn rejects_unclosed_braces() {
        let error = parse("\"a\"\n{\n\"b\" \"c\"\n").unwrap_err();
        assert_eq!(error.message, "unclosed brace");
    }
}
