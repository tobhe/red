mod parser;
mod error;

extern crate nom;

use crate::parser::{
	parse_command,
	Command,
	Line
};
use crate::error::CommandError;
use std::str::Lines;
use std::fs;
use std::env;
use std::io;
use std::convert::TryFrom;

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
		println!("Newline: {}", newline);
		Err(CommandError::new("Invalid address"))
	}
}

fn handle_command(s: &mut State, c: Command) -> Result<()> {
	match c {
		Command::Goto(l) => {
			s.line = update_line(s, l)?
		},
		Command::Print(r) =>  {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer.lines().skip(usize::try_from(from)?)
			    .take(usize::try_from(to)? - usize::try_from(from)? + 1)
			    .for_each(|s| {println!("{}", s);});
		},
		Command::Number(r) =>  {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer.lines().skip(usize::try_from(from)?).enumerate()
			    .take(usize::try_from(to)? - usize::try_from(from)? + 1)
			    .for_each(|(i, s)| {println!("{:<4} {}", i + 1, s);});
		},
		_ => {}
	}
	Ok(())
}

fn main() {
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
				parse_command(&input)
				    .or(Err(CommandError::new("Invalid Command")))
				    .and_then(|(_, i)| {handle_command(&mut state, i)})
				    .unwrap_or_else(|e| {println!("? ({})", e);});
			}
			Err(error) => {
			    println!("error: {}", error);
			}
		}
	}
}
