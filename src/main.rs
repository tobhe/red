mod parser;
mod error;

use crate::parser::{
	parse_command,
	Command,
	Line,
	Range
};
use crate::error::CommandError;
use std::str::Lines;
use std::fs;
use std::env;
use std::io::{self, Write};
use std::convert::TryFrom;

type Result<T> = std::result::Result<T, CommandError>;

struct State {
	line: u32,
	total: usize,
	mode: Mode,
	buffer: String,
	prompt: bool,
}

enum Mode {
	CommandMode,
	InsertMode,
}

// Load new file
fn load_file(s: &str) -> State {
	let buf = fs::read_to_string(&s)
	    .expect("Something went wrong reading the file");
	println!("{}", buf.as_bytes().len());
	State {line: 0, total: buf.lines().count(),
	    mode: Mode::CommandMode, buffer: buf, prompt: false}
}

fn update_line(s: &mut State, l: Line) -> Result<u32> {
	let newline = match l {
		Line::Abs(c) => {
			if c < 0 {
				u32::try_from(i64::try_from(s.total)?
				    + i64::from(c))?
			} else {
				u32::try_from(c)?
			}
		},
		Line::Rel(c) => {
			u32::try_from(i64::from(s.line)
			    + i64::from(c))?
		}
	};
	if newline < u32::try_from(s.total)? {
		Ok(newline)
	} else {
		Err(CommandError::new("Invalid address"))
	}
}

fn handle_command(s: &mut State, c: (Range, Option<Command>)) -> Result<()> {
	match c {
		(l, None) => {
			s.line = update_line(s, l.from)?
		},
		(r, Some(Command::Print)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer.lines().skip(usize::try_from(from)?)
			    .take(usize::try_from(to)? - usize::try_from(from)? + 1)
			    .for_each(|s| {println!("{}", s);});
		},
		(r, Some(Command::Number)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer.lines().enumerate().skip(usize::try_from(from)?)
			    .take(usize::try_from(to)? - usize::try_from(from)? + 1)
			    .for_each(|(i, s)| {println!("{:<4} {}", i + 1, s);});
		},
		(_, Some(Command::Prompt)) => {
			s.prompt = !s.prompt; 
		},
/*
		(l, Command::Goto) => {
			s.line = update_line(s, l.from)?
		},
*/
		_ => {
			return Err(CommandError::new("Invalid command"));
		}
	}
	Ok(())
}

fn main() {
	let args: Vec<String> = env::args().collect();
	let mut state = if args.len() == 2 {
		load_file(&args[1])
	} else {
		State {line: 0, total: 0, mode: Mode::CommandMode,
		    buffer: String::from(""), prompt: false}
	};

	loop {
		let mut input = String::new();
		if (state.prompt == true) {
			print!("* ");
		}
		io::stdout().flush().unwrap();
		match io::stdin().read_line(&mut input) {
			Ok(n) => {	
				parse_command(&input)
				    .or(Err(CommandError::new("Invalid Command")))
				    .and_then(|(_, t)| {handle_command(&mut state, t)})
				    .unwrap_or_else(|e| {println!("? ({})", e);});
			}
			Err(error) => {
			    println!("error: {}", error);
			}
		}
	}
}
