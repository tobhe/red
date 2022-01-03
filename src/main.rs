/*
 * Copyright (c) 2021-2022 Tobias Heider <me@tobhe.de>
 *
 * Permission to use, copy, modify, and distribute this software for any
 * purpose with or without fee is hereby granted, provided that the above
 * copyright notice and this permission notice appear in all copies.
 *
 * THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
 * WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
 * MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
 * ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
 * WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
 * ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
 * OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
 */

mod error;
mod parser;

use crate::error::CommandError;
use crate::parser::{parse_command, parse_terminator, Command, Line, Range};
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

type Result<T> = std::result::Result<T, CommandError>;

struct State {
	changed: bool,
	line: usize,
	total: usize,
	mode: Mode,
	buffer: String,
	prompt: bool,
	verbose: bool,
	file: String,
}

impl Default for State {
	fn default() -> Self {
		State {
			changed: false,
			line: 0,
			total: 1,
			mode: Mode::CommandMode,
			buffer: String::from(""),
			prompt: false,
			verbose: false,
			file: String::from(""),
		}
	}
}

#[derive(PartialEq)]
enum Mode {
	CommandMode,
	InsertMode(usize, String),
}

fn read_file(f: &str) -> Result<State> {
	let buf = fs::read_to_string(&f).map_err(|_| CommandError::new("invalid path"))?;
	println!("{}", buf.as_bytes().len());
	let total = buf.lines().count();
	let last = if total > 0 { total - 1 } else { total };
	Ok(State {
		line: last,
		total: total,
		file: String::from(f),
		buffer: buf,
		..State::default()
	})
}

fn write_file(s: &State, f: &str) -> Result<()> {
	fs::write(f, s.buffer.as_str()).map_err(|_| CommandError::new("invalid path"))?;
	Ok(())
}

fn update_line(s: &mut State, l: Line) -> Result<usize> {
	let newline = match l {
		Line::Abs(c) => {
			if c < 0 {
				usize::try_from(i32::try_from(s.total)? + c)?
			} else {
				usize::try_from(c)?
			}
		}
		Line::Rel(c) => usize::try_from(i32::try_from(s.line)? + c)?,
	};
	if newline < s.total {
		Ok(newline)
	} else {
		Err(CommandError::new("invalid address"))
	}
}

fn handle_command(s: &mut State, c: (Range, Option<Command>)) -> Result<()> {
	match c {
		(l, None) => {
			if l.from != l.to {
				return Err(CommandError::new("Expected single line"));
			}
			s.line = update_line(s, l.from)?;
			println!(
				"{}",
				s.buffer
					.lines()
					.nth(s.line)
					.ok_or(CommandError::new("invalid address"))?
			);
		}
		(r, Some(Command::Change)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			let head = s.buffer.lines().take(from);
			let tail = s.buffer.lines().skip(to + 1);
			s.buffer = head.chain(tail).fold(String::new(), |e, l| e + l + "\n");
			s.total = s.buffer.lines().count();
			s.mode = Mode::InsertMode(from, String::new());
		}
		(_, Some(Command::CurLine)) => {
			println!("{}", s.line + 1);
		}
		(r, Some(Command::Delete)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			let head = s.buffer.lines().take(from);
			let tail = s.buffer.lines().skip(to + 1);
			s.buffer = head.chain(tail).fold(String::new(), |e, l| e + l + "\n");
			s.total = s.buffer.lines().count();
			s.changed = true;
		}
		(_, Some(Command::Edit(f))) => {
			if s.changed == true {
				s.changed = false;
				return Err(CommandError::new("warning: file modified"));
			}
			if let Some(f) = f {
				*s = read_file(&f)?;
				s.file = f;
			} else {
				*s = read_file(&s.file)?;
			}
		}
		(_, Some(Command::Exec(c))) => {
			process::Command::new("sh")
				.arg("-c")
				.arg(c)
				.status()
				.map_err(|_| CommandError::new("Command failed"))?;
			println!("!");
		}
		(_, Some(Command::File(f))) => {
			s.file = f;
		}
		(l, Some(Command::Append)) => {
			if l.from != l.to {
				return Err(CommandError::new("Expected single line"));
			}
			let line = update_line(s, l.from)?;
			s.mode = Mode::InsertMode(line + 1, String::new());
		}
		(l, Some(Command::Insert)) => {
			if l.from != l.to {
				return Err(CommandError::new("Expected single line"));
			}
			let line = update_line(s, l.from)?;
			s.mode = Mode::InsertMode(line, String::new());
		}
		(r, Some(Command::Number)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer
				.lines()
				.enumerate()
				.skip(from)
				.take(to - from + 1)
				.for_each(|(i, s)| {
					println!("{}\t{}", i + 1, s);
				});
		}
		(r, Some(Command::Print)) => {
			let from = update_line(s, r.from)?;
			let to = update_line(s, r.to)?;
			s.buffer
				.lines()
				.skip(from)
				.take(to - from + 1)
				.for_each(|s| {
					println!("{}", s);
				});
		}
		(_, Some(Command::Prompt)) => {
			s.prompt = !s.prompt;
		}
		(_, Some(Command::Help)) => {
			s.verbose = !s.verbose;
		}
		(_, Some(Command::Write(f))) => {
			if let Some(f) = f {
				write_file(s, &f)?;
			} else {
				write_file(s, &s.file)?;
			};
		}
		(_, Some(Command::Quit)) => {
			if s.changed == true {
				s.changed = false;
				return Err(CommandError::new("warning: file modified"));
			}
			process::exit(0);
		}
		_ => {
			return Err(CommandError::new("invalid command"));
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
					.or(Err(CommandError::new("invalid command")))
					.and_then(|(_, t)| handle_command(&mut state, t))
					.unwrap_or_else(|e| {
						println!("?");
						if state.verbose == true {
							println!("{}", e);
						}
					});
			}
			Mode::InsertMode(l, ref mut b) => {
				if parse_terminator(&input).is_ok() {
					// Write to buf
					let head = state.buffer.lines().take(l);
					let tail = state.buffer.lines().skip(l);
					state.buffer = head
						.chain(b.lines())
						.chain(tail)
						.fold(String::new(), |s, l| s + l + "\n");
					state.total = state.buffer.lines().count();
					state.mode = Mode::CommandMode;
					state.changed = true;
				} else {
					b.push_str(&input);
				}
			}
		}
	}
}
