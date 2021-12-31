mod parser;
mod error;

use crate::parser::{
	parse_command,
	parse_terminator,
	Command,
	Line,
	Range
};
use crate::error::CommandError;
use std::fs;
use std::env;
use std::io::{self, Write};
use std::convert::TryFrom;
use std::process;

type Result<T> = std::result::Result<T, CommandError>;

struct State {
	line: u32,
	total: usize,
	mode: Mode,
	buffer: String,
	prompt: bool,
}

#[derive(PartialEq)]
enum Mode {
	CommandMode,
	InsertMode(usize, String),
}

fn read_file(f: &str) -> Result<State> {
	let buf = fs::read_to_string(&f).map_err(|e| CommandError::new("Invalid path"))?;
	println!("{}", buf.as_bytes().len());
	Ok(State {line: 0, total: buf.lines().count(),
	    mode: Mode::CommandMode, buffer: buf, prompt: false})
}

fn write_file(s: &State, f: &str) -> Result<()> {
	fs::write(f, s.buffer.as_str()).map_err(|e| CommandError::new("Invalid path"))?;
	Ok(())
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
			if l.from != l.to {
				return Err(CommandError::new("Expected single line"));
			}
			s.line = update_line(s, l.from)?;
			println!("{}", s.buffer.lines().nth(usize::try_from(s.line)?)
			    .ok_or(CommandError::new("Invalid Address"))?);
		},
		(_, Some(Command::CurLine)) => {
			println!("{}", s.line + 1);
		},
		(r, Some(Command::Delete)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			let head = s.buffer.lines().take(usize::try_from(from)?);
			let tail = s.buffer.lines().skip(usize::try_from(to)? + 1);
			s.buffer = head.chain(tail).fold(String::new(),
			    |e, l| e + l + "\n");
			s.total = s.buffer.lines().count();
		},
		(_, Some(Command::Edit(f))) => {
			*s = read_file(&f)?;
		},
		(l, Some(Command::Insert)) => {
			if l.from != l.to {
				return Err(CommandError::new("Expected single line"));
			}
			let line = update_line(s, l.from)?;
			s.mode = Mode::InsertMode(usize::try_from(line)?, String::new());
		},
		(r, Some(Command::Number)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer.lines().enumerate().skip(usize::try_from(from)?)
			    .take(usize::try_from(to)? - usize::try_from(from)? + 1)
			    .for_each(|(i, s)| {println!("{:<4} {}", i + 1, s);});
		},
		(r, Some(Command::Print)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer.lines().skip(usize::try_from(from)?)
			    .take(usize::try_from(to)? - usize::try_from(from)? + 1)
			    .for_each(|s| {println!("{}", s);});
		},
		(_, Some(Command::Prompt)) => {
			s.prompt = !s.prompt;
		},
		(_, Some(Command::Write(f))) => {
			write_file(s, &f)?;
		},
		(_, Some(Command::Quit)) => {
			process::exit(0);
		},
		_ => {
			return Err(CommandError::new("Invalid command"));
		}
	}
	Ok(())
}

fn main() {
	let args: Vec<String> = env::args().collect();
	let mut state = if args.len() == 2 {
		read_file(&args[1]).unwrap()
	} else {
		State {line: 0, total: 0, mode: Mode::CommandMode,
		    buffer: String::from(""), prompt: false}
	};

	loop {
		let mut input = String::new();
		if state.mode == Mode::CommandMode && state.prompt == true {
			print!("* ");
			io::stdout().flush().unwrap();
		}
		if let Err(_) = io::stdin().read_line(&mut input) {
			// XXX: error?
			continue;
		}
		match state.mode {
			Mode::CommandMode => {
				parse_command(&input)
				    .or(Err(CommandError::new("Invalid Command")))
				    .and_then(|(_, t)| {handle_command(&mut state, t)})
				    .unwrap_or_else(|e| {println!("? ({})", e);});
			},
			Mode::InsertMode(l, ref mut b) => {
				if parse_terminator(&input).is_ok() {
					// Write to buf
					let head = state.buffer.lines().take(l);
					let tail = state.buffer.lines().skip(l);
					state.buffer = head.chain(b.lines()).chain(tail).fold(String::new(), |s, l| s + l + "\n");
					state.total = state.buffer.lines().count();
					state.mode = Mode::CommandMode;
				} else {
					b.push_str(&input);
				}
			},
		}
	}
}
