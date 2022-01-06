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
use crate::parser::{
	parse_command, parse_terminator, print_flag_set, Address, AddressRange, Command, PrintFlag,
};
use std::convert::TryFrom;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::iter;
use std::process;

use regex::Regex;

type Result<T> = std::result::Result<T, CommandError>;

struct State {
	buffer: Vec<String>,
	changed: bool,
	file: String,
	last_match: (Option<usize>, Option<regex::Regex>),
	line: usize,
	marks: [Option<usize>; 26],
	prompt: bool,
	total: usize,
	verbose: bool,
}

impl Default for State {
	fn default() -> Self {
		State {
			buffer: Vec::new(),
			changed: false,
			file: String::from(""),
			last_match: (None, None),
			line: 0,
			marks: [None; 26],
			prompt: false,
			total: 1,
			verbose: false,
		}
	}
}

fn buf_as_string(b: &Vec<String>) -> String {
	b.iter().fold(String::new(), |e, l| e + l + "\n")
}

fn read_to_vec(f: &str) -> Result<Vec<String>> {
	let file = File::open(f).map_err(|_| CommandError::new("invalid path"))?;
	let lines = io::BufReader::new(file).lines();
	let mut b = Vec::new();
	b.extend(lines.filter_map(|s| s.ok()));
	// XXX: There must be an easier way
	let mut len = 0;
	for line in b.iter() {
		len = len + line.bytes().count() + 1;
	}
	println!("{}", len);
	Ok(b)
}

fn read_file(f: &str) -> Result<State> {
	// let buf = fs::read_to_string(&f)
	// println!("{}", buf.as_bytes().len());
	let buf = read_to_vec(&f)?;
	let total = buf.len();
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
	fs::write(f, buf_as_string(&s.buffer)).map_err(|_| CommandError::new("invalid path"))?;
	Ok(())
}

fn buffer_insert(s: &mut State, line: usize, buf: Vec<String>) {
	s.line = s.line + buf.len();
	s.buffer.splice(line..line, buf);
	s.total = s.buffer.len();
	s.changed = true;
}

fn update_line(s: &mut State, l: Address) -> Result<usize> {
	let newline = match l {
		Address::Abs(c) => {
			if c < 0 {
				usize::try_from(i32::try_from(s.total)? + c)?
			} else {
				usize::try_from(c)?
			}
		}
		Address::Rel(c) => usize::try_from(i32::try_from(s.line)? + c)?,
		Address::Mark(m) => s.marks[usize::from(m)].ok_or(CommandError::new("invalid mark"))?,
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
		.iter()
		.enumerate()
		.skip(from)
		.take(to - from + 1)
		.for_each(fun);
}

fn find_regex(s: &mut State, regex: Option<&String>, forward: bool) -> Result<(usize, usize)> {
	let (i, r) = if let Some(re) = regex {
		s.last_match.1 = Some(Regex::new(&re).map_err(|_| CommandError::new("invalid regex"))?);
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

fn handle_insert() -> Vec<String> {
	let mut buf = Vec::new();
	let mut input = String::new();
	loop {
		io::stdin().read_line(&mut input).unwrap();
		if parse_terminator(&input).is_ok() {
			return buf;
		} else {
			buf.push(String::from(&input[..input.len() - 1]));
		}
		input.clear();
	}
}

fn handle_command(
	s: &mut State,
	c: (Option<AddressRange>, Option<Command>, PrintFlag),
) -> Result<()> {
	let (range, command, mut flags) = c;

	let (from, to) = match range {
		Some(AddressRange::Range(f, t)) => {
			let from = update_line(s, f)?;
			let to = update_line(s, t)?;
			if from > to {
				return Err(CommandError::new("invalid address"));
			}
			(from, to)
		}
		Some(AddressRange::Next(re)) => {
			if command.is_none() {
				flags = print_flag_set(flags, PrintFlag::Print);
			}
			find_regex(s, re.as_ref(), true)?
		}
		Some(AddressRange::Prev(re)) => {
			if command.is_none() {
				flags = print_flag_set(flags, PrintFlag::Print);
			}
			find_regex(s, re.as_ref(), false)?
		}
		None => (
			update_line(s, Address::Rel(0))?,
			update_line(s, Address::Rel(0))?,
		),
	};

	match command {
		None => {
			if flags == PrintFlag::None {
				s.line = is_line(from, to)?;
				flags = print_flag_set(flags, PrintFlag::Print);
			}
		}
		Some(com @ Command::Append) | Some(com @ Command::Insert) => {
			let line = match com {
				Command::Append => is_line(from, to)? + 1,
				Command::Insert => is_line(from, to)?,
				_ => unreachable!(),
			};
			buffer_insert(s, line, handle_insert());
		}
		Some(com @ Command::Change) | Some(com @ Command::Delete) => {
			s.buffer.splice(from..(to + 1), iter::empty::<String>());
			let old = s.total;
			s.total = s.buffer.len();
			if s.line > to {
				s.line = s.line - (old - s.total);
			}
			match com {
				Command::Change => {
					buffer_insert(s, from, handle_insert());
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
		Some(Command::Mark(m)) => {
			s.marks[usize::from(m)] = Some(is_line(from, to)?);
		}
		Some(Command::Prompt) => {
			s.prompt = !s.prompt;
		}
		Some(Command::Read(f)) => {
			let buf = match f {
				Some(f) => read_to_vec(&f),
				_ => read_to_vec(&s.file),
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
			s.changed = false;
		}
		Some(Command::Quit) => {
			if s.changed == true {
				s.changed = false;
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
		read_file(&args[1]).unwrap_or(Default::default())
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
			.and_then(|(_, t)| handle_command(&mut state, t))
			.unwrap_or_else(|e| {
				println!("?");
				if state.verbose == true {
					println!("{}", e);
				}
			});
	}
}
