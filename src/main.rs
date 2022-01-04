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
use crate::parser::{parse_command, parse_terminator, Address, Command, Line, PrintFlag};
use std::convert::TryFrom;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

use regex::Regex;

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
	last_match: (Option<usize>, Option<regex::Regex>),
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
			last_match: (None, None),
		}
	}
}

#[derive(PartialEq)]
enum Mode {
	CommandMode,
	InsertMode(usize, String, PrintFlag),
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

fn print_range(s: &State, from: usize, to: usize, flags: PrintFlag) {
	let fun = if flags == PrintFlag::Number {
		|(i, s)| println!("{}\t{}", i + 1, s)
	} else {
		|(_, s)| println!("{}", s)
	};
	s.buffer
		.lines()
		.enumerate()
		.skip(from)
		.take(to - from + 1)
		.for_each(fun);
}

fn handle_command(s: &mut State, c: (Option<Address>, Option<Command>, PrintFlag)) -> Result<()> {
	let (range, command, mut flags) = c;

	let (from, to) = match range {
		Some(Address::Range(f, t)) => {
			let from = update_line(s, f)?;
			let to = update_line(s, t)?;
			if from > to {
				return Err(CommandError::new("invalid address"));
			}
			(from, to)
		}
		Some(Address::Regex(re)) => {
			let (i, r) = if let Some(re) = re {
				s.last_match.1 =
					Some(Regex::new(&re).map_err(|_| CommandError::new("invalid regex"))?);
				(s.line, s.last_match.1.as_ref().unwrap())
			} else {
				(
					s.last_match
						.0
						.ok_or(CommandError::new("no previous search"))?,
					s.last_match
						.1
						.as_ref()
						.ok_or(CommandError::new("no previous search"))?,
				)
			};
			let head = s.buffer.lines().enumerate().take(i + 1);
			let tail = s.buffer.lines().enumerate().skip(i + 1);
			let (i, _) = tail
				.chain(head)
				.find(|(_, l)| r.is_match(l))
				.ok_or(CommandError::new("no match"))?;
			s.last_match.0 = Some(i);

			// Print if no command was given
			if command.is_none() {
				if flags == PrintFlag::None {
					flags = PrintFlag::Print;
				}
			}
			(i, i)
		}
		None => (update_line(s, Line::Rel(0))?, update_line(s, Line::Rel(0))?),
	};

	match command {
		None => {
			if flags == PrintFlag::None {
				if from != to {
					return Err(CommandError::new("Expected single line"));
				}
				s.line = to;
				println!(
					"{}",
					s.buffer
						.lines()
						.nth(s.line)
						.ok_or(CommandError::new("invalid address"))?
				);
			}
		}
		Some(com @ Command::Append) | Some(com @ Command::Insert) => {
			if from != to {
				return Err(CommandError::new("Expected single line"));
			}
			let line = match com {
				Command::Append => to + 1,
				Command::Insert => to,
				_ => unreachable!(),
			};
			s.mode = Mode::InsertMode(line, String::new(), flags);
			return Ok(());
		}
		Some(com @ Command::Change) | Some(com @ Command::Delete) => {
			let head = s.buffer.lines().take(from);
			let tail = s.buffer.lines().skip(to + 1);
			s.buffer = head.chain(tail).fold(String::new(), |e, l| e + l + "\n");
			let old = s.total;
			s.total = s.buffer.lines().count();
			if s.line > to {
				s.line = s.line - (old - s.total);
			}
			match com {
				Command::Change => {
					s.mode = Mode::InsertMode(from, String::new(), flags);
					return Ok(());
				}
				Command::Delete => s.changed = true,
				_ => unreachable!(),
			}
		}
		Some(Command::CurLine) => {
			println!("{}", s.line + 1);
		}
		Some(Command::Edit(f)) => {
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
		Some(Command::Exec(c)) => {
			process::Command::new("sh")
				.arg("-c")
				.arg(c)
				.status()
				.map_err(|_| CommandError::new("Command failed"))?;
			println!("!");
		}
		Some(Command::File(f)) => {
			s.file = f;
		}
		Some(Command::Help) => {
			s.verbose = !s.verbose;
		}
		Some(Command::Prompt) => {
			s.prompt = !s.prompt;
		}
		Some(Command::Write(f)) => {
			if let Some(f) = f {
				write_file(s, &f)?;
			} else {
				write_file(s, &s.file)?;
			};
			s.changed = false;
		}
		Some(Command::Quit) => {
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
	if flags != PrintFlag::None {
		print_range(s, from, to, flags);
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
			Mode::InsertMode(l, ref mut b, flags) => {
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
					if flags != PrintFlag::None {
						print_range(&state, l, l, flags);
					}
				} else {
					b.push_str(&input);
				}
			}
		}
	}
}
