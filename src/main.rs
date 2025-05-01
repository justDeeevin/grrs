use clap::{ArgAction, Parser};
use color_eyre::{eyre::Context, owo_colors::OwoColorize};
use crossterm::{
    ExecutableCommand,
    cursor::{self, MoveToColumn},
};
use regex::Regex;

use std::{
    fs::File,
    io::{BufRead, BufReader, IsTerminal, Read},
    path::PathBuf,
};

/// Search for a pattern in a file and display the lines that contain it.
#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    /// The pattern to look for
    pattern: String,

    /// The path to the file to read.
    /// If no path is specified, grrs will read from stdin.
    path: Option<PathBuf>,

    /// Whether to interpret the pattern as a regex
    #[clap(long, short)]
    regex: bool,

    /// Whether to use ANSI formatting.
    /// By default, grrs will use formatting if in a terminal. Setting this option
    /// will override this behavior.
    #[clap(long, short, action = ArgAction::Set)]
    color: Option<bool>,

    /// Disable the printing of capture group names (if using regex)
    no_group_names: bool,
    #[clap(long, requires = "regex")]
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = Cli::parse();

    let mut reader: BufReader<Box<dyn Read>> = match args.path {
        Some(path) => BufReader::new(Box::new(File::open(path)?)),
        None => BufReader::new(Box::new(std::io::stdin())),
    };

    if args.pattern.is_empty() {
        std::io::copy(&mut reader, &mut std::io::stdout())?;
        return Ok(());
    }

    let color = args
        .color
        .unwrap_or_else(|| std::io::stdout().is_terminal());

    if args.regex {
        let re = Regex::new(&args.pattern).context("Failed to build regex")?;
        for (line_number, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("Failed to read line {line_number}"))?;

            let Some(captures) = re.captures(&line) else {
                // No match
                continue;
            };

            if !color {
                println!("{line}");
                continue;
            }

            let mut iter = captures.iter().peekable();

            // captures[0] contains the full match
            let full_range = iter.next().unwrap().unwrap().range();

            print!("{}", &line[..full_range.start]);

            if let Some(Some(first)) = iter.peek() {
                print!(
                    "{}",
                    (&line[full_range.start..first.range().start]).red().bold()
                );
            } else {
                // No capture groups
                print!("{}", (&line[full_range.clone()]).red().bold());
                println!("{}", &line[full_range.end..]);
                continue;
            }

            let captures = iter.collect::<Vec<_>>();
            for window in captures.iter().flatten().collect::<Vec<_>>().windows(2) {
                let first = window[0];
                let range = first.range();
                print!("{}", (&line[range.clone()]).red().bold().underline());
                let Some(second) = window.get(1) else {
                    continue;
                };
                print!("{}", (&line[range.end..second.range().start]).red().bold())
            }
            let last_range = captures.last().unwrap().unwrap().range();
            print!("{}", (&line[last_range.clone()]).red().bold().underline());
            print!("{}", (&line[last_range.end..full_range.end]).red().bold());
            println!("{}", &line[full_range.end..]);
            let mut stdout = std::io::stdout();
            if !args.no_group_names {
                for (i, name) in re
                    .capture_names()
                    .skip(1)
                    .enumerate()
                    // collect is necessary because CaptureNames is not a double-ended
                    // iterator
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                {
                    let name = match name {
                        Some(name) => name.to_string(),
                        None => format!("group {i}"),
                    };
                    let range = captures.get(i).unwrap().unwrap().range();
                    if cursor::position()?.0 >= range.start as u16 {
                        println!();
                    }
                    stdout.execute(MoveToColumn(range.start as u16))?;
                    let out = format!("^{name}");
                    print!("{}", out.italic().blue());
                }
                println!();
            }
        }
    } else {
        for (line_number, line) in reader.lines().enumerate() {
            let line = line.with_context(|| format!("Failed to read line {line_number}"))?;

            let Some(start) = line.find(&args.pattern) else {
                // No match
                continue;
            };

            if !color {
                println!("{line}");
                continue;
            }

            print!("{}", &line[..start]);
            print!(
                "{}",
                (&line[start..(start + args.pattern.len())]).red().bold()
            );
            println!("{}", &line[(start + args.pattern.len())..]);
        }
    }

    Ok(())
}
