use std::fmt;
use anyhow::{anyhow, bail, Result};

pub enum Token {
    Add,
    Sub,
    Comma,
    Semicolon,
    Dollar,
    Dot,
    Ident(String),
    Str(String),
    Num(usize),
    Eol,
}

pub struct Lexer<'a> {
    line: &'a str,
    start: usize,
    current: usize,
    eol: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            line: input,
            start: 0,
            current: 0,
            eol: false,
        }
    }

    fn is_eol(&self) -> bool {
        self.eol
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(c) = self.line[self.current..].chars().next() {
            self.current += c.len_utf8();
            Some(c)
        }
        else {
            self.eol = true;
            None
        }
    }

    fn move_curr(&mut self, c: char) {
        self.current += c.len_utf8();
    }

    fn peek(&self) -> Option<char> {
        let c = self.line[self.current..].chars().next()?;
        Some(c)
    }

    fn is_digit(&self, c: char) -> bool {
        c >= '0' && c <= '9'
    }

    fn is_hex_digit(&self, c: char) -> bool {
        self.is_digit(c) || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')
    }

    fn is_alpha(&self, c: char) -> bool {
        (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
    }

    fn is_alpha_numeric(&self, c: char) -> bool {
        self.is_alpha(c) || self.is_digit(c)
    }

    fn identifier(&mut self) -> Result<Token> {
        loop {
            if let Some(c) = self.peek() {
                if self.is_alpha_numeric(c) {
                    self.move_curr(c);
                    continue
                }
            }
            break;
        }
        let name = self.line[self.start..self.current].to_string();
        Ok(Token::Ident(name))
    }

    fn string(&mut self) -> Result<Token> {
        loop {
            if let Some(c) = self.peek() {
                if c != '"' {
                    self.move_curr(c);
                    continue;
                }
                self.move_curr(c);
                let value = (&self.line[self.start + 1..self.current - 1])
                    .to_string();
                return Ok(Token::Str(value));
            }
            else {
                bail!("Unterminated string.");
            }
        }
    }

    fn integer(&mut self) -> Result<Token> {
        loop {
            if let Some(c) = self.peek() {
                if self.is_digit(c) {
                    self.move_curr(c);
                    continue
                }
            }
            let value = &self.line[self.start..self.current];
            let num = usize::from_str_radix(&value, 10)?;
            return Ok(Token::Num(num));
        }
    }

    fn integer_hex(&mut self) -> Result<Token> {
        loop {
            if let Some(c) = self.peek() {
                if self.is_hex_digit(c) {
                    self.move_curr(c);
                    continue
                }
            }
            let value = &self.line[self.start + 2..self.current];
            let num = usize::from_str_radix(&value, 16)?;
            return Ok(Token::Num(num));
        }
    }

    fn probe_hex(&mut self) -> Result<Token> {
        if let Some(c) = self.peek() {
            if c == 'x' {
                self.move_curr(c);
                return self.integer_hex();
            }
        }
        // just normal integer
        self.integer()
    }

    fn comment(&mut self) -> Result<Token> {
        self.eol = true;
        Ok(Token::Eol)
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Token::Add => write!(f, "ADD"),
            Token::Sub => write!(f, "SUB"),
            Token::Comma => write!(f, "COMMA"),
            Token::Semicolon => write!(f, "SEMICOLON"),
            Token::Dollar => write!(f, "DOLLAR"),
            Token::Dot => write!(f, "DOT"),
            Token::Str(ref value) => write!(f, "STR {value}"),
            Token::Ident(ref name) => write!(f, "IDENT {name}"),
            Token::Num(value) => write!(f, "INT {value}"),
            Token::Eol => write!(f, "EOL"),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.is_eol() {
            return None;
        }
        loop {
            self.start = self.current;
            let c = self.advance();
            let ch = match c {
                Some(c) => c,
                None => return Some(Ok(Token::Eol)),
            };
            return match ch {
                ' ' | '\t' => continue,
                '+' => Some(Ok(Token::Add)),
                '-' => Some(Ok(Token::Sub)),
                ':' => Some(Ok(Token::Semicolon)),
                ',' => Some(Ok(Token::Comma)),
                '$' => Some(Ok(Token::Dollar)),
                '.' => Some(Ok(Token::Dot)),
                '"' => Some(self.string()),
                '#' => Some(self.comment()),
                '0' => Some(self.probe_hex()),
                '1'..='9' => Some(self.integer()),
                'a'..='z' | 'A'..='Z' | '_' => Some(self.identifier()),
                _ => Some(Err(anyhow!("Unknown character '{}'", ch))),
            }
        }
    }
}

