mod parser;

extern crate nom;

use crate::parser::{
	parse_command,
	Command,
	Line
};
use std::str::Lines;
use std::fmt;
use std::error;
use std::error::Error;
use std::fs;
use std::env;
use std::io;
use std::convert::TryFrom;
use std::num::TryFromIntError;

type Result<T> = std::result::Result<T, CommandError>;

struct State {
	line: u32,
	total: usize,
	mode: Mode,
	buffer: String,
}

enum Mode {
	CommandMode,
	InsertMode,
}

#[derive(Debug)]
struct CommandError {
	details: String,
}

impl CommandError {
	fn new(msg: &str) -> CommandError {
		CommandError{details: msg.to_string()}
	}
}

impl fmt::Display for CommandError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f,"{}",self.details)
	}
}

impl Error for CommandError {
	fn description(&self) -> &str {
		&self.details
	}
}

impl From<TryFromIntError> for CommandError {
	fn from(err: TryFromIntError) -> Self {
		CommandError::new(err.description())
	}
}

fn update_line(s: &mut State, l: Line) -> Result<u32> {
	let mut newline = 0;
	match l {
		Line::Abs(c) => {
			if c < 0 {
				newline = u32::try_from(i64::from(s.line)
				    + i64::from(c))?;
			} else {
				newline = u32::try_from(c)?;
			}
		}
		Line::Rel(c) => {
			newline = u32::try_from(i64::from(s.line)
			    + i64::from(c))?;
		}
	};
	if newline < u32::try_from(s.total)? {
		Ok(newline)
	} else {
		Err(CommandError::new("Invalid address"))
	}
}

fn handle_command(s: &mut State, c: Command) -> Result<()> {
	println!("success {:?}", c);
	match c {
		Command::Goto(l) => {
		    s.line = update_line(s, l)?
		},
		_ => {}
	}
	Ok(())
}

fn main() {
	let stdin = io::stdin();

	let args: Vec<String> = env::args().collect();
	let mut state = if args.len() == 2 {
		// println!("Loading file: {}", &args[1]);
		let buf = fs::read_to_string(&args[1])
		    .expect("Something went wrong reading the file");
		State {line: 0, total: buf.lines().count(),
		    mode: Mode::CommandMode, buffer: buf}
	} else {
		State {line: 0, total: 0, mode: Mode::CommandMode,
		    buffer: String::from("")}
	};

	loop {
		let mut input = String::new();
		match io::stdin().read_line(&mut input) {
			Ok(n) => {	
				match parse_command(&input) {
					Ok((input, n)) => {
						match handle_command(&mut state, n) {
							Err(_) => println!("?"),
							_ => {}
						}
					}
					Err(_) => println!("?"),
				}
			}
			Err(error) => {
			    println!("error: {}", error);
			}
		}
	}
}
