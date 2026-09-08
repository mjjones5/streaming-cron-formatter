use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::process::ExitCode;

mod normalize;

// Reads via BufReader::lines(), which pulls one line at a time off the
// underlying handle. A crontab full of comments and a multi-gigabyte piped
// log of schedule strings should both run in constant memory, not O(input).
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    let reader: Box<dyn BufRead> = match args.first() {
        Some(path) => match File::open(path) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("cronfmt: cannot open {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => Box::new(BufReader::new(io::stdin())),
    };

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut had_error = false;

    for (idx, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("cronfmt: read error at line {}: {e}", idx + 1);
                had_error = true;
                break;
            }
        };

        match normalize::normalize_line(&line) {
            Ok(normalized) => {
                if let Err(e) = writeln!(out, "{normalized}") {
                    eprintln!("cronfmt: write error: {e}");
                    return ExitCode::FAILURE;
                }
            }
            Err(err) => {
                // Pass the original line through unchanged so a bad line
                // doesn't silently disappear from the output.
                eprintln!("cronfmt: line {}: {err}", idx + 1);
                had_error = true;
                if let Err(e) = writeln!(out, "{line}") {
                    eprintln!("cronfmt: write error: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
