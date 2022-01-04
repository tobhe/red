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

extern crate nom;

use nom::{
	branch::alt,
	character::complete::{anychar, char, i32, newline, none_of},
	character::is_newline,
	combinator::opt,
	error::{Error, ErrorKind},
	multi::{many0, many1},
	sequence::{preceded, terminated, tuple},
	Err, IResult, InputTakeAtPosition,
};

pub enum Address {
	Range(Line, Line),     // (.,.)		Address range
	Regex(Option<String>), // /re/		Next line containing the regex
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Line {
	Rel(i32),
	Abs(i32),
}

/*
 * Commands: All commands are single characters, some require additional
 * parameters.
 */
#[derive(Debug)]
pub enum Command {
	Append,                // (.)a		Append text to the buffer
	Change,                // (.,.)c	Change line in buffer
	CurLine,               // =		Print line number
	Delete,                // (.,.)d	Delete lines
	Edit(Option<String>),  // e file	Edit file
	Exec(String),          // !cmd		Execute command
	File(String),          // f file        Set default filename
	Help,                  // H		Toggle error explanations
	Insert,                // (.)i		Insert text before current line
	Prompt,                // P		Enable * prompt
	Read,                  // ($)r		Reads file to after the addressed line
	Write(Option<String>), // w file	Write buffer to file
	Quit,                  // q		Quit
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrintFlag {
	None,
	Print,
	Number,
}

pub fn parse_command(i: &str) -> IResult<&str, (Option<Address>, Option<Command>, PrintFlag)> {
	let (i, (r, c, f)) = terminated(
		tuple((
			opt(parse_address),
			opt(alt((parse_command_char, parse_file_command, parse_exec))),
			many0(parse_flag),
		)),
		newline,
	)(i)?;
	Ok((
		i,
		(
			r,
			c,
			f.into_iter().fold(PrintFlag::None, |fs, flag| {
				if fs == PrintFlag::None || (fs == PrintFlag::Print && flag == PrintFlag::Number) {
					flag
				} else {
					fs
				}
			}),
		),
	))
}

// Commands
fn parse_command_char(i: &str) -> IResult<&str, Command> {
	let (i, c) = anychar(i)?;
	let cmd = match c {
		'a' => Command::Append,
		'c' => Command::Change,
		'd' => Command::Delete,
		'H' => Command::Help,
		'i' => Command::Insert,
		'P' => Command::Prompt,
		'q' => Command::Quit,
		'r' => Command::Read,
		'=' => Command::CurLine,
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, cmd))
}

fn parse_flag(i: &str) -> IResult<&str, PrintFlag> {
	let (i, c) = anychar(i)?;
	let f = match c {
		'n' => PrintFlag::Number,
		'p' => PrintFlag::Print,
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, f))
}

fn parse_file_command(i: &str) -> IResult<&str, Command> {
	let (i, (c, s)) = tuple((anychar, opt(preceded(char(' '), parse_path))))(i)?;
	let cmd = match c {
		'e' => Command::Edit(s.map(ToString::to_string)),
		'f' => Command::File(
			s.ok_or(Err::Error(Error::new("line", ErrorKind::Char)))?
				.to_string(),
		),
		'w' => Command::Write(s.map(ToString::to_string)),
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, cmd))
}

fn parse_exec(i: &str) -> IResult<&str, Command> {
	let (i, s) = preceded(char('!'), parse_path)(i)?;
	Ok((i, Command::Exec(s.to_string())))
}

fn parse_path(i: &str) -> IResult<&str, &str> {
	i.split_at_position1_complete(|item| is_newline(item as u8), ErrorKind::Fail)
}

// Insert Mode
pub fn parse_terminator(i: &str) -> IResult<&str, Command> {
	let (i, _) = terminated(char('.'), newline)(i)?;
	Ok((i, Command::Append))
}

// Helpers
fn parse_address(i: &str) -> IResult<&str, Address> {
	alt((
		parse_range_special,
		parse_range_tuple,
		parse_range_simple,
		parse_search,
	))(i)
}

fn parse_search(i: &str) -> IResult<&str, Address> {
	let (i, s) = preceded(char('/'), opt(many1(none_of("/\n"))))(i)?;
	let (i, _) = opt(char('/'))(i)?;
	Ok((i, Address::Regex(s.map(|re| re.into_iter().collect()))))
}

fn parse_range_special(i: &str) -> IResult<&str, Address> {
	let (i, c) = anychar(i)?;
	let range = match c {
		'%' | ',' => Address::Range(Line::Abs(0), Line::Abs(-1)),
		';' => Address::Range(Line::Rel(0), Line::Abs(-1)),
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, range))
}

fn parse_range_tuple(i: &str) -> IResult<&str, Address> {
	let (i, f) = parse_line(i)?;
	let (i, _) = char(',')(i)?;
	let (i, t) = parse_line(i)?;
	Ok((i, Address::Range(f, t)))
}

fn parse_range_simple(i: &str) -> IResult<&str, Address> {
	let (i, f) = parse_line(i)?;
	Ok((i, Address::Range(f, f)))
}

fn parse_sign(i: &str) -> IResult<&str, char> {
	alt((char('+'), char('-')))(i)
}

fn parse_line(i: &str) -> IResult<&str, Line> {
	alt((parse_line_regular, parse_line_special))(i)
}

fn parse_line_special(i: &str) -> IResult<&str, Line> {
	let (i, c) = anychar(i)?;
	let line = match c {
		'.' => Line::Rel(0),
		'$' => Line::Abs(-1),
		'+' => Line::Rel(1),
		'-' | '^' => Line::Rel(-1),
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, line))
}

fn parse_line_regular(i: &str) -> IResult<&str, Line> {
	let pref = parse_sign(i);
	let (i, o) = i32(i)?;
	match pref {
		Ok(_) => Ok((i, Line::Rel(o))),
		_ => {
			if o > 0 {
				Ok((i, Line::Abs(o - 1)))
			} else {
				return Err(Err::Error(Error::new("line", ErrorKind::Fail)));
			}
		}
	}
}
