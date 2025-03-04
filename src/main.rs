
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::path;
use std::fs::{File};
use std::io::prelude::*;
use std::io::{copy, SeekFrom, Seek, Read, Write, BufReader};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::convert::TryInto;
use crc;
use std::error::Error;

mod lexer;
mod parser;

/// A tool to combine binary files
#[derive(Parser)]
struct Cli {
    /// The path to the file to read layout
    layout: path::PathBuf,
    /// The path to the file to output
    output: path::PathBuf,
    /// Constants
    #[arg(short = 'D', value_parser = parse_consts::<String, String>)]
    defines: Vec<(String, String)>,
}

fn main() -> Result<()>
{
    let args = Cli::parse();

    let rpath = &args.layout;
    let inf = File::open(rpath)
        .with_context(
            || format!("could not open file `{}`", rpath.display())
        )?;

    let reader = BufReader::new(inf);

    let consts: HashMap<String, String> = args.defines.into_iter().collect();

    let mut variables: HashMap<String, usize> = HashMap::new();
    let wpath = &args.output;
    let mut outf = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(true)
        .open(wpath)
        .with_context(
            || format!("could not create file `{}`", wpath.display())
        )?;

    for (index, buf) in reader.lines().enumerate() {
        if let Ok(sline) = buf {
            let line_no = index + 1;
            let lex = lexer::Lexer::new(&sline);
            let arr: Result<Vec<lexer::Token>> = lex.collect();
            let tokens: Vec<lexer::Token> = arr
                .with_context(|| format!("line {line_no}"))?;

            let mut parser = parser::Parser::new(&tokens);
            let expr = parser.parse();
            if let Some(e) = expr {
                interpret(&consts, &mut variables, &mut outf, e?)
                    .with_context(|| format!("line {line_no}"))?;
            }
        }
    }
    Ok(())
}

pub fn valid_const_name(s: &str) -> Result<&str> {
    let mut c = s.chars();
    while let Some(c) = c.next() {
        match c {
            'A'..='Z' | '_' => continue,
            _ => bail!("Invalid name of key '{s}'")
        }
    }
    Ok(s)
}

fn parse_consts<T, U>(s: &str) -> Result<(T, U), Box<dyn Error + Send + Sync + 'static>>
where
    T: std::str::FromStr,
    T::Err: Error + Send + Sync + 'static,
    U: std::str::FromStr,
    U::Err: Error + Send + Sync + 'static,
{
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=value: no '=' found in '{s}'"))?;
    Ok((valid_const_name(&s[..pos])?.parse()?, s[pos + 1..].parse()?))
}

fn evaluate(consts: &HashMap<String, String>,
            vars: &HashMap<String, usize>,
            expr: &parser::Expr) -> Result<usize>
{
    match expr {
        parser::Expr::Literal(value) => Ok(*value),
        parser::Expr::Variable(name) => {
            if let Some(value) = vars.get(name) {
                Ok(*value)
            }
            else {
                bail!("Undefined variable {name}");
            }
        },
        parser::Expr::Const(name) => {
            if let Some(value) = consts.get(name) {
                Ok(usize::from_str_radix(&value, 10)?)
            }
            else {
                bail!("Undefined constant {name}");
            }
        },
        parser::Expr::Binary {
            op: lexer::Token::Add,
            left,
            right,
        } => {
            let op1 = evaluate(consts, vars, left)?;
            let op2 = evaluate(consts, vars, right)?;
            Ok(op1 + op2)
        },
        parser::Expr::Binary {
            op: lexer::Token::Sub,
            left,
            right,
        } => {
            let op1 = evaluate(consts, vars, left)?;
            let op2 = evaluate(consts, vars, right)?;
            Ok(op1 - op2)
        },
        _ => bail!("Invalid expression"),
    }
}

fn interpret<F>(consts: &HashMap<String, String>,
                vars: &mut HashMap<String, usize>, outf: &mut F,
                expr: parser::Expr) -> Result<()>
where
    F: Seek + Read + Write,
{
    if let parser::Expr::Statement {offset, variable, func} = expr {
        let pos = evaluate(consts, vars, &offset)?;

        if let parser::Expr::Call {callee, args} = *func {
            if callee == "file" {
                if args.len() != 1 {
                    bail!("Error number of arguments");
                }
                let path = match &args[0] {
                    parser::Expr::Str(path) => path,
                    parser::Expr::Const(name) => consts.get(name)
                        .ok_or(anyhow!("Undefined constant {name}"))?,
                    _ => bail!("Expected string or constant")
                };
                let f = File::open(path)
                    .with_context(
                        || format!("Could not open file {path}")
                    )?;
                let mut reader = BufReader::new(f);
                outf.seek(SeekFrom::Start(pos.try_into()?))?;
                let length = copy(&mut reader, outf)?.try_into()?;

                add_variables(vars, &variable, pos, length)?;
            }
            else if callee == "crc16" {
                if args.len() != 2 {
                    bail!("Error number of arguments")
                }
                let addr = evaluate(consts, vars, &args[0])?;
                let length = evaluate(consts, vars, &args[1])?;

                outf.seek(SeekFrom::Start(addr.try_into()?))?;
                let mut bin = vec![0; length.try_into()?];
                outf.read(&mut bin)?;

                let crc = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
                let result = crc.checksum(&bin).to_le_bytes();
                outf.seek(SeekFrom::Start(pos.try_into()?))?;
                let _ = outf.write(&result[..2])?;

                add_variables(vars, &variable, pos, length)?;
            }
            else {
                bail!("Unknown function name '{}'", callee);
            }

            return Ok(());
        }
    }
    bail!("Invalid statement");
}

fn add_variables(vars: &mut HashMap<String, usize>, name: &str, addr: usize,
                 size: usize) -> Result<()>
{
    if name != "_" {
        let key_start = format!("{name}.start");
        if vars.contains_key(&key_start) {
            bail!("Variables with name '{name}' already defined");
        }

        vars.insert(key_start, addr);
        vars.insert(format!("{name}.size"), size);
        vars.insert(format!("{name}.end"), addr + size);
    }

    Ok(())
}
