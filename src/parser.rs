use std::fmt;
use anyhow::{anyhow, bail, Result};

use crate::lexer::Token;

pub enum Expr {
    Statement { offset: Box<Expr>, variable: String, func: Box<Expr> },
    Binary { op: Token, left: Box<Expr>, right: Box<Expr> },
    Call { callee: String, args: Vec<Expr> },
    Variable(String),
    Str(String),
    Literal(usize),
}

pub struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens: tokens,
            current: 0,
        }
    }

    fn peek(&self) -> Option<&Token> {
        if self.current < self.tokens.len() {
            let token = &self.tokens[self.current];
            Some(token)
        }
        else {
            None
        }
    }

    fn cons_semicolon(&mut self) -> Result<()> {
        match self.peek() {
            Some(Token::Semicolon) => {
                self.current += 1;
                Ok(())
            },
            _ => bail!("Expected semicolon."),
        }
    }

    fn cons_ident(&mut self) -> Result<String> {
        let name = match self.peek() {
            Some(Token::Ident(name)) => {
                name.to_string()
            },
            _ => bail!("Expected semicolon."),
        };
        self.current += 1;
        Ok(name)
    }

    fn cons_arg(&mut self) -> Option<Result<Expr>> {
        match self.peek() {
            Some(Token::Comma) => { self.current += 1; },
            None => return Some(Err(anyhow!("Unexpected end of input"))),
            _ => return None
        };

        Some(self.expr())
    }

    pub fn parse(&mut self) -> Option<Result<Expr>> {
        match self.peek() {
            Some(Token::Eol) => None,
            Some(_) => Some(self.statement()),
            None => Some(Err(anyhow!("Unexpected end of input"))),
        }
    }

    fn statement(&mut self) -> Result<Expr> {
        let offset = self.expr()?;
        self.cons_semicolon()?;
        let var_name = self.cons_ident()?;
        self.cons_semicolon()?;
        let func_name = self.cons_ident()?;

        let mut func_args = Vec::new();
        while let Some(arg) = self.cons_arg() {
            func_args.push(arg?);
        }

        let func = Expr::Call {
            callee: func_name,
            args: func_args,
        };

        match self.peek() {
            Some(Token::Eol) => { self.current += 1; },
            _ => bail!("End of line expected"),
        };

        Ok(Expr::Statement {
            offset: Box::new(offset),
            variable: var_name,
            func: Box::new(func),
        })
    }

    fn expr(&mut self) -> Result<Expr> {
        let expr = self.primary()?;

        match self.peek() {
            Some(Token::Add) => {
                self.current += 1;
                Ok(Expr::Binary {
                    op: Token::Add,
                    left: Box::new(expr),
                    right: Box::new(self.expr()?)
                })
            }
            Some(Token::Sub) => {
                self.current += 1;
                Ok(Expr::Binary {
                    op: Token::Sub,
                    left: Box::new(expr),
                    right: Box::new(self.expr()?)
                })
            },
            _ => Ok(expr)
        }
    }

    fn primary(&mut self) -> Result<Expr> {
        let val = match self.peek() {
            Some(Token::Num(value)) => {
                Expr::Literal(*value)
            },
            Some(Token::Str(value)) => {
                Expr::Str(value.to_string())
            },
            Some(Token::Dollar) => return self.variable(),
            _ => bail!("Unexpected primary token"),
        };
        self.current += 1;
        Ok(val)
    }

    fn variable(&mut self) -> Result<Expr> {
        self.current += 1; // skip dollar
        let mut var_name = String::new();
        if let Some(Token::Ident(name)) = self.peek() {
            var_name.push_str(name);
            self.current += 1;
        }
        else {
            bail!("Expected identifier");
        }
        if let Some(Token::Dot) = self.peek() {
            var_name.push_str(".");
            self.current += 1;
        }
        else {
            bail!("Expected dot");
        }
        if let Some(Token::Ident(name)) = self.peek() {
            var_name.push_str(name);
            self.current += 1;
        }
        else {
            bail!("Expected identifier");
        }
        Ok(Expr::Variable(var_name))
    }
}


impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Literal(value) => write!(f, "{value}"),
            Expr::Variable(name) => write!(f, "${name}"),
            Expr::Str(value) => write!(f, "'{value}'"),
            Expr::Binary {
                op,
                left,
                right,
            } => {
                write!(f, "{left} {op} {right}")
            },
            Expr::Statement {
                offset,
                variable,
                func,
            } => {
                write!(f, "{offset}:{variable}:{func}")
            },
            Expr::Call {
                callee,
                args,
            } => {
                write!(f, "{callee}")?;
                for arg in args {
                    write!(f, ",{arg}")?
                };
                Ok(())
            }
        }
    }
}


// https://github.com/jonhoo/lox/blob/master/src/parse.rs
// pub fn parse(tokens: &mut Peekable<Lexer>) {
//    let item = tokens.peek();
//    if let Some(Ok(t)) = item {
//        println!("{}", t);
//    }
//}

