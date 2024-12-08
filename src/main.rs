use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{copy, SeekFrom, Seek, Read, Write, BufReader};
use std::path;
use std::convert::TryInto;
use std::collections::HashMap;
use crc;

#[derive(Debug)]
struct Entry<'a> {
    addr: u64,
    name: &'a str,
    func: &'a str,
    args: Vec<&'a str>,
}

/// A tool to combine binary files
#[derive(Parser)]
struct Cli {
    /// The path to the file to read layout
    layout: path::PathBuf,
    /// The path to the file to output
    output: path::PathBuf,
}

fn main() -> Result<()> {
    let args = Cli::parse();

    let mut variables: HashMap<String, u64> = HashMap::new();

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
    let rpath = &args.layout;
    let inf = File::open(rpath)
        .with_context(
            || format!("could not open file `{}`", rpath.display())
        )?;

    let reader = BufReader::new(inf);

    for (index, buf) in reader.lines().enumerate() {
        if let Ok(sline) = buf {
            let line = sline.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }

            let entry = Entry::from_str(&line)?;
            process_entry(&mut variables, &mut outf, &entry)
                .with_context(
                    || format!("Failed on line {}", index + 1)
                )?;
        }
    }

    println!("{:?}", variables);

    Ok(())
}

fn process_entry<F>(vars: &mut HashMap<String, u64>, outf: &mut F, entry: &Entry) -> Result<()>
where
    F: Seek + Read + Write,
{
    let mut length: u64 = 0;
    let mut var_name: String = entry.name.to_string();
    var_name.push_str(".start");
    vars.insert(var_name, entry.addr);

    if entry.func == "file" {
        if entry.args.len() != 1 {
            bail!("Error number of arguments");
        }
        let f = File::open(entry.args[0])
            .with_context(
                || format!("Could not open file {}", entry.args[0])
            )?;
        let mut reader = BufReader::new(f);
        outf.seek(SeekFrom::Start(entry.addr))?;
        length = copy(&mut reader, outf)?;
    }
    else if entry.func == "crc16" {
        if entry.args.len() != 2 {
            bail!("Error number of arguments")
        }

        let addr = unpack_arg(&vars, &entry.args[0])?;
        length = unpack_arg(&vars, &entry.args[1])?;

        outf.seek(SeekFrom::Start(addr))?;
        let mut bin = vec![0; length.try_into()?];
        outf.read(&mut bin)?;

        let crc = crc::Crc::<u16>::new(&crc::CRC_16_IBM_SDLC);
        let result = crc.checksum(&bin).to_le_bytes();
        outf.seek(SeekFrom::Start(entry.addr))?;
        let _ = outf.write(&result[..2])?;
    }
    else {
        bail!("Unknown function name '{}'", entry.func);
    }

    let mut var_name: String = entry.name.to_string();
    var_name.push_str(".size");
    vars.insert(var_name, length);

    Ok(())
}

fn parse_uint(s: &str) -> Result<u64> {
    let hex_prefix = "0x";
    let mut value = s;
    let mut base = 10;

    if s.starts_with(hex_prefix) {
        value = &value[2..];
        base = 16;
    }

    Ok(u64::from_str_radix(&value, base)?)
}

fn unpack_arg(vars: &HashMap<String, u64>, arg: &str) -> Result<u64> {
    if arg.starts_with("$") {
        if let Some(&value) = vars.get(&arg[1..]) {
            return Ok(value)
        }
        Err(anyhow!("Missing variable: {}", arg))
    }
    else {
        parse_uint(arg)
    }
}

impl<'a> Entry<'a> {
    fn from_str(line: &str) -> Result<Entry> {
        let values = line.split(':').map(|el| el.trim()).collect::<Vec<&str>>();

        if values.len() != 3 {
            bail!("Error number values");
        }

        if values[2].is_empty() {
            bail!("Function name cannot be empty");
        }

        let address = parse_uint(&values[0])?;

        let func = values[2]
            .split(",")
            .map(|el| el.trim())
            .collect::<Vec<&str>>();

        Ok(Entry {
            addr: address,
            name: &values[1],
            func: &func[0],
            args: func[1..].to_vec(),
        })
    }
}
