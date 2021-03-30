use std::convert::TryInto;
use std::fs::File;
use std::io::copy;
use std::io::prelude::*;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;
use std::num::ParseIntError;
use std::u64;

#[derive(Debug)]
struct Entry<'a> {
    addr: u64,
    name: &'a str,
    func: &'a str,
    args: Vec<&'a str>,
}

#[derive(Debug)]
enum AppError {
    NumberOfArgs,
    Address,
    Func,
    IOErr,
    FileNotFound,
}

fn main() {
    let out = File::create("result.bin").unwrap();
    let mut writer = BufWriter::new(out);

    let script = File::open("bincomb.txt").unwrap();
    let mut reader = BufReader::new(script);
    let mut line = String::new();
    let mut line_no = 0;

    loop {
        line_no += 1;
        line.clear();

        let len = reader.read_line(&mut line).unwrap();
        let line = line.trim();

        if len == 0 {
            break;
        }

        if line.is_empty() || line.starts_with("#") {
            continue;
        }

        let entry = match Entry::from_str(&line) {
            Ok(v) => v,
            Err(err) => panic!("Error on line {}: {:?}", line_no, err),
        };

        process_entry(&mut writer, &entry).unwrap();
    }
}

fn process_entry<W>(buf: &mut W, entry: &Entry) -> Result<usize, AppError>
where
    W: Seek + Write,
{
    let mut length: usize = 0;

    buf.seek(SeekFrom::Start(entry.addr))
        .map_err(|_| AppError::IOErr)?;

    if entry.func == "file" {
        if entry.args.len() != 1 {
            return Err(AppError::NumberOfArgs);
        }
        let f = File::open(entry.args[0]).map_err(|_| AppError::FileNotFound)?;
        let mut reader = BufReader::new(f);
        length = copy(&mut reader, buf)
            .map_err(|_| AppError::IOErr)?
            .try_into()
            .unwrap();
    }

    Ok(length)
}

fn parse_uint(s: &str) -> Result<u64, ParseIntError> {
    let hex_prefix = "0x";
    let mut value = s;
    let mut base = 10;

    if s.starts_with(hex_prefix) {
        value = &value[2..];
        base = 16;
    }

    u64::from_str_radix(&value, base)
}

impl<'a> Entry<'a> {
    fn from_str(line: &str) -> Result<Entry, AppError> {
        let values = line.split(':').map(|el| el.trim()).collect::<Vec<&str>>();

        if values.len() != 3 {
            return Err(AppError::NumberOfArgs);
        }

        if values[2].is_empty() {
            return Err(AppError::Func);
        }

        let address = parse_uint(&values[0]).map_err(|_| AppError::Address)?;

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

#[test]
fn test_entry_with_not_enougth_args() {
    match Entry::from_str("0x0:name") {
        Ok(_) => assert!(false),
        Err(AppError::NumberOfArgs) => assert!(true),
        Err(_) => assert!(false),
    }
}

#[test]
fn test_entry_without_func() {
    match Entry::from_str("0x0:name:") {
        Ok(_) => assert!(false),
        Err(AppError::Func) => assert!(true),
        Err(_) => assert!(false),
    }
}

#[test]
fn test_entry_address() -> Result<(), AppError> {
    match Entry::from_str("0xads:name:func") {
        Ok(_) => assert!(false),
        Err(AppError::Address) => assert!(true),
        Err(_) => assert!(false),
    }
    assert!(Entry::from_str("0x20:name:func")?.addr == 0x20);
    assert!(Entry::from_str("100:name:func")?.addr == 100);
    Ok(())
}
