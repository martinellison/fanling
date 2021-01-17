/*! prepare android ADB input commands */
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
//#[macro_use]

use std::path::PathBuf;
use structopt::StructOpt;
#[derive(Debug, StructOpt)]
#[structopt(name = "android-keys", about = "prepare android ADB input commands .")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    /// prefix characters e.g. adb shell input
    #[structopt(short, long = "prefix")]
    prefix: String,
}
fn main() {
    let opt = Opt::from_args();
    let lines = match read_lines(opt.input) {
        Ok(it) => it,
        _ => return,
    };
    let mut quoter = Quoter::new(opt.prefix);
    for line in lines {
        if let Ok(s) = line {
            quoter.fix_line(s);
        }
    }
}
/** An `Quoter` quotes a string */
struct Quoter {
    prefix: String,
    chars: String,
}
impl Quoter {
    /** Create a new Quoter */
    fn new(prefix: String) -> Self {
        Self {
            prefix,
            chars: "".to_string(),
        }
    }
    fn fix_line(&mut self, s: String) {
        for c in s.chars() {
            match c {
                ' ' => self.put_chars(62),  /* space */
                '\'' => self.put_chars(75), // escape single quotes
                '\"' | '&' | '|' | '$' => {
                    self.chars.push('\\');
                    self.chars.push(c);
                } //  escape
                ';' => self.put_chars(74),  //  escape semicolons
                '(' => self.put_chars(71),  // escape opening parentheses
                ')' => self.put_chars(72),  //  escape closing parentheses

                _ => self.chars.push(c),
            }
        }
        self.put_chars(66) /* new line */
;
    }
    fn put_chars(&mut self, n: u8) {
        println!("{} text {}", &self.prefix, self.chars.clone());
        self.chars = "".to_string();
        println!("{} keyevent {}", &self.prefix, n)
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
