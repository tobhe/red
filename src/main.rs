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

mod buffer;
mod error;
mod parser;

use crate::buffer::Buffer;
use crate::error::CommandError;
use crate::parser::{
	parse_command, parse_terminator, print_flag_set, Address, AddressRange, Command, PrintFlag,
};
use std::convert::TryFrom;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::iter::{self, FromIterator};
use std::process;

use regex::Regex;

type Result<T> = std::result::Result<T, CommandError>;

struct State {
	buffer: Buffer,
	file: String,
	last_match: (Option<usize>, Option<regex::Regex>),
	prompt: bool,
	verbose: bool,
}

impl Default for State {
	fn default() -> Self {
		State {
			buffer: Buffer::new(),
			file: String::from(""),
			last_match: (None, None),
			prompt: false,
			verbose: false,
		}
	}
}

fn read_to_buffer(f: &str) -> Result<Buffer> {
	let file = File::open(f).map_err(|_| CommandError::new("invalid path"))?;
	let lines = io::BufReader::new(file).lines();
	Ok(Buffer::from_iter(lines.filter_map(|s| s.ok())))
}

fn read_file(s: &State, f: &str) -> Result<State> {
	let buf = read_to_buffer(&f)?;

	// Print bytes
	// XXX: There must be an easier way
	let mut len = 0;
	for line in buf.iter() {
		len = len + line.bytes().count() + 1;
	}
	println!("{}", len);

	Ok(State {
		file: String::from(f),
		buffer: buf,
		prompt: s.prompt,
		verbose: s.verbose,
		..State::default()
	})
}

fn write_file(s: &State, f: &str) -> Result<()> {
	fs::write(f, s.buffer.to_string()).map_err(|_| CommandError::new("invalid path"))?;
	Ok(())
}

fn buffer_insert(s: &mut State, line: usize, buf: Buffer) {
	s.buffer.replace_iter(line..line, buf);
}

fn line_to_index(s: &mut State, l: Address) -> Result<usize> {
	let newline = match l {
		Address::Abs(c) => {
			if c < 0 {
				usize::try_from(i32::try_from(s.buffer.len())? + c)?
			} else {
				usize::try_from(c)?
			}
		}
		Address::Rel(c) => usize::try_from(i32::try_from(s.buffer.curline)? + c)?,
		Address::Mark(m) => {
			s.buffer.marks[usize::from(m)].ok_or(CommandError::new("invalid mark"))?
		}
	};
	Ok(newline)
}

fn print_range(s: &State, from: usize, to: usize, flags: PrintFlag) {
	let fun = if flags == PrintFlag::Number {
		|(i, s)| println!("{}\t{}", i + 1, s)
	} else {
		|(_, s)| println!("{}", s)
	};
	s.buffer
		.iter()
		.enumerate()
		.skip(from)
		.take(to - from + 1)
		.for_each(fun);
}

fn find_regex(s: &mut State, regex: Option<&String>, forward: bool) -> Result<(usize, usize)> {
	let (i, r) = if let Some(re) = regex {
		s.last_match.1 = Some(Regex::new(&re).map_err(|_| CommandError::new("invalid regex"))?);
		(s.buffer.curline, s.last_match.1.as_ref().unwrap())
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
	let (i, _) = match forward {
		true => s
			.buffer
			.iter()
			.enumerate()
			.skip(i + 1)
			.chain(s.buffer.iter().enumerate().take(i + 1))
			.find(|(_, l)| r.is_match(l))
			.ok_or(CommandError::new("no match"))?,
		false => s
			.buffer
			.iter()
			.enumerate()
			.skip(i)
			.chain(s.buffer.iter().enumerate().take(i))
			.rfind(|(_, l)| r.is_match(l))
			.ok_or(CommandError::new("no match"))?,
	};
	s.last_match.0 = Some(i);

	// Print if no command was given
	Ok((i, i))
}

fn is_line(from: usize, to: usize) -> Result<usize> {
	if from != to {
		return Err(CommandError::new("Expected single line"));
	}
	Ok(to)
}

fn is_valid(s: &State, i: usize) -> Result<usize> {
	if i < s.buffer.len() {
		Ok(i)
	} else {
		Err(CommandError::new("invalid address"))
	}
}

fn input_to_buffer(buf: &mut Buffer) {
	let mut input = String::new();
	loop {
		io::stdin().read_line(&mut input).unwrap();
		if parse_terminator(&input).is_ok() {
			return;
		} else {
			buf.push(String::from(&input[..input.len() - 1]));
		}
		input.clear();
	}
}

fn extract_addr_range(s: &mut State, range: Option<AddressRange>) -> Result<(usize, usize)> {
	match range {
		Some(AddressRange::Range(f, t)) => {
			let from = line_to_index(s, f)?;
			let to = line_to_index(s, t)?;
			if from > to {
				return Err(CommandError::new("invalid address"));
			}
			Ok((from, to))
		}
		Some(AddressRange::Next(re)) => {
			//			if command.is_none() {
			//				flags = print_flag_set(flags, PrintFlag::Print);
			//			}
			Ok(find_regex(s, re.as_ref(), true)?)
		}
		Some(AddressRange::Prev(re)) => {
			//			if command.is_none() {
			//				flags = print_flag_set(flags, PrintFlag::Print);
			//			}
			Ok(find_regex(s, re.as_ref(), false)?)
		}
		None => Ok((
			line_to_index(s, Address::Rel(0))?,
			line_to_index(s, Address::Rel(0))?,
		)),
	}
}

fn exec_command(
	s: &mut State,
	c: (Option<AddressRange>, Option<Command>, PrintFlag),
) -> Result<()> {
	let (range, mut command, mut flags) = c;

	let (from, to) = extract_addr_range(s, range)?;

	// Get input if needed
	match command {
		Some(Command::Append(ref mut b))
		| Some(Command::Insert(ref mut b))
		| Some(Command::Change(ref mut b)) => input_to_buffer(b),
		_ => {}
	};

	match command {
		None => {
			is_valid(s, from)?;
			is_valid(s, to)?;
			if flags == PrintFlag::None {
				s.buffer.curline = is_line(from, to)?;
				flags = print_flag_set(flags, PrintFlag::Print);
			}
		}
		Some(com @ Command::Append(_)) | Some(com @ Command::Insert(_)) => {
			let line = is_line(from, to)?;

			match com {
				Command::Append(b) => buffer_insert(s, is_valid(s, line + 1).unwrap_or(line), b),
				Command::Insert(b) => buffer_insert(s, line, b),
				_ => unreachable!(),
			}
		}
		Some(com @ Command::Change(_)) | Some(com @ Command::Delete) => {
			is_valid(s, from)?;
			is_valid(s, to)?;
			match com {
				Command::Change(b) => s.buffer.replace_iter(from..(to + 1), b),
				Command::Delete => {
					s.buffer
						.replace_iter(from..(to + 1), iter::empty::<String>());
					// Delete is special as it wants a current line after the deletion
					s.buffer.curline =
						is_valid(s, s.buffer.curline + 1).unwrap_or(s.buffer.curline);
				}
				_ => unreachable!(),
			};
		}
		Some(Command::CurLine) => {
			println!("{}", s.buffer.curline + 1);
		}
		Some(Command::Edit(f)) => {
			if s.buffer.changed == true {
				s.buffer.changed = false;
				return Err(CommandError::new("warning: file modified"));
			}
			if let Some(f) = f {
				*s = read_file(&s, &f)?;
				s.file = f;
			} else {
				*s = read_file(&s, &s.file)?;
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
		Some(Command::Mark(m)) => {
			is_valid(s, from)?;
			is_valid(s, to)?;
			s.buffer.marks[usize::from(m)] = Some(is_line(from, to)?);
		}
		Some(Command::Prompt) => {
			s.prompt = !s.prompt;
		}
		Some(Command::Read(f)) => {
			let buf = match f {
				Some(f) => read_to_buffer(&f),
				_ => read_to_buffer(&s.file),
			}
			.map_err(|_| CommandError::new("invalid path"))?;
			buffer_insert(s, is_line(from, to)? + 1, buf);
		}
		Some(Command::Write(f)) => {
			if let Some(f) = f {
				write_file(s, &f)?;
			} else {
				write_file(s, &s.file)?;
			};
		}
		Some(Command::Quit) => {
			if s.buffer.changed == true {
				s.buffer.changed = false;
				return Err(CommandError::new("warning: file modified"));
			}
			process::exit(0);
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
		read_file(&Default::default(), &args[1]).unwrap_or(Default::default())
	} else {
		Default::default()
	};

	loop {
		let mut input = String::new();
		if state.prompt == true {
			print!("* ");
			io::stdout().flush().unwrap();
		}
		io::stdin().read_line(&mut input).unwrap();
		parse_command(&input)
			.or(Err(CommandError::new("invalid command")))
			.and_then(|(_, t)| exec_command(&mut state, t))
			.unwrap_or_else(|e| {
				println!("?");
				if state.verbose == true {
					println!("{}", e);
				}
			});
	}
}
