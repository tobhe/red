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
	line: usize,
	total: usize,
	mode: Mode,
	buffer: String,
	prompt: bool,
	file: String,
}

impl Default for State {
	fn default() -> Self{State {line: 0, total: 0, mode: Mode::CommandMode,
	    buffer: String::from(""), prompt: false,
	    file: String::new()}}
}

#[derive(PartialEq)]
enum Mode {
	CommandMode,
	InsertMode(usize, String),
}

fn read_file(f: &str) -> Result<State> {
	let buf = fs::read_to_string(&f)
	    .map_err(|_| CommandError::new("Invalid path"))?;
	println!("{}", buf.as_bytes().len());
	let total = buf.lines().count();
	let last = if total > 0 {
		total - 1
	} else {
		total
	};
	Ok(State {line: last, total: total, mode: Mode::CommandMode,
	    buffer: buf, prompt: false, file: String::from(f)})
}

fn write_file(s: &State, f: &str) -> Result<()> {
	fs::write(f, s.buffer.as_str())
	    .map_err(|_| CommandError::new("Invalid path"))?;
	Ok(())
}

fn update_line(s: &mut State, l: Line) -> Result<usize> {
	let newline = match l {
		Line::Abs(c) => {
			if c < 0 {
				usize::try_from(i32::try_from(s.total)?
				    + c)?
			} else {
				usize::try_from(c)?
			}
		},
		Line::Rel(c) => {
			usize::try_from(i32::try_from(s.line)?  + c)?
		}
	};
	if newline < s.total {
		Ok(newline)
	} else {
		Err(CommandError::new("Invalid address"))
	}
}

fn handle_command(s: &mut State, c: (Range, Option<Command>)) -> Result<()> {
	match c {
		(l, None) => {
			if l.from != l.to {
				return Err(
				    CommandError::new("Expected single line"));
			}
			s.line = update_line(s, l.from)?;
			println!("{}", s.buffer.lines().nth(s.line)
			    .ok_or(CommandError::new("Invalid Address"))?);
		},
		(r, Some(Command::Change)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			let head = s.buffer.lines().take(from);
			let tail = s.buffer.lines().skip(to + 1);
			s.buffer = head.chain(tail).fold(String::new(),
			    |e, l| e + l + "\n");
			s.total = s.buffer.lines().count();
			s.mode = Mode::InsertMode(from, String::new());
		},
		(_, Some(Command::CurLine)) => {
			println!("{}", s.line + 1);
		},
		(r, Some(Command::Delete)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			let head = s.buffer.lines().take(from);
			let tail = s.buffer.lines().skip(to + 1);
			s.buffer = head.chain(tail).fold(String::new(),
			    |e, l| e + l + "\n");
			s.total = s.buffer.lines().count();
		},
		(_, Some(Command::Edit(f))) => {
			*s = read_file(&f)?;
		},
		(_, Some(Command::Exec(c))) => {
			process::Command::new("sh").arg("-c").arg(c).status()
			    .map_err(|_| CommandError::new("Command failed"))?;
			println!("!");
		},
		(l, Some(Command::Insert)) => {
			if l.from != l.to {
				return Err(CommandError::new("Expected single line"));
			}
			let line = update_line(s, l.from)?;
			s.mode = Mode::InsertMode(line, String::new());
		},
		(r, Some(Command::Number)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer.lines().enumerate().skip(from)
			    .take(to - from + 1)
			    .for_each(|(i, s)| {println!("{:<4} {}", i + 1, s);});
		},
		(r, Some(Command::Print)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer.lines().skip(from).take(to - from + 1)
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
		read_file(&args[1]).unwrap_or(Default::default())
	} else {
		Default::default()
	};

	loop {
		let mut input = String::new();
		if state.mode == Mode::CommandMode && state.prompt == true {
			print!("* ");
			io::stdout().flush().unwrap();
		}
		io::stdin().read_line(&mut input).unwrap();
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
					state.buffer = head.chain(b.lines())
					    .chain(tail).fold(String::new(),
					    |s, l| s + l + "\n");
					state.total = state.buffer.lines().count();
					state.mode = Mode::CommandMode;
				} else {
					b.push_str(&input);
				}
			},
		}
	}
}
