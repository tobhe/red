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

use crate::buffer::Buffer;
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

pub enum AddressRange {
	Range(Address, Address), // (.,.)	Address range
	Next(Option<String>),    // /re/	Next line containing the regex
	Prev(Option<String>),    // ?re?	Previous line containing the regex
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Address {
	Abs(i32), // N		Nth line in the buffer
	Rel(i32), // +-N	Nth next or previous line
	Mark(u8), // 'x		Line previosly marked with x
}

/*
 * Commands: All commands are single characters, some require additional
 * parameters.
 */
#[derive(Debug)]
pub enum Command {
	Append(Buffer),        // (.)a		Append text to the buffer
	Change(Buffer),        // (.,.)c	Change line in buffer
	CurLine,               // =		Print line number
	Delete,                // (.,.)d	Delete lines
	Edit(Option<String>),  // e file	Edit file
	Exec(String),          // !cmd		Execute command
	File(String),          // f file        Set default filename
	Help,                  // H		Toggle error explanations
	Insert(Buffer),        // (.)i		Insert text before current line
	Mark(u8),              // kx		Marks a line with a lower case letter
	Prompt,                // P		Enable * prompt
	Read(Option<String>),  // ($)r		Reads file to after the addressed line
	Write(Option<String>), // w file	Write buffer to file
	Quit,                  // q		Quit
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PrintFlag {
	None,
	Print,
	Number,
}

pub fn print_flag_set(fs: PrintFlag, flag: PrintFlag) -> PrintFlag {
	if fs == PrintFlag::None || (fs == PrintFlag::Print && flag == PrintFlag::Number) {
		flag
	} else {
		fs
	}
}

pub fn parse_command(i: &str) -> IResult<&str, (Option<AddressRange>, Option<Command>, PrintFlag)> {
	let (i, (r, c, f)) = terminated(
		tuple((
			opt(parse_address_range),
			opt(alt((
				parse_simple_cmd,
				parse_mark_cmd,
				parse_file_cmd,
				parse_exec_cmd,
			))),
			many0(parse_flag),
		)),
		newline,
	)(i)?;
	Ok((
		i,
		(
			r,
			c,
			f.into_iter()
				.fold(PrintFlag::None, |fs, flag| print_flag_set(fs, flag)),
		),
	))
}

// Commands
fn parse_simple_cmd(i: &str) -> IResult<&str, Command> {
	let (i, c) = anychar(i)?;
	let cmd = match c {
		'a' => Command::Append(Buffer::new()),
		'c' => Command::Change(Buffer::new()),
		'd' => Command::Delete,
		'H' => Command::Help,
		'i' => Command::Insert(Buffer::new()),
		'P' => Command::Prompt,
		'q' => Command::Quit,
		'=' => Command::CurLine,
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, cmd))
}

fn parse_mark_cmd(i: &str) -> IResult<&str, Command> {
	let (i, c) = preceded(char('m'), anychar)(i)?;
	let c = c as u8;
	if c > 0x60 && c < 0x7b {
		Ok((i, Command::Mark(c - 0x61)))
	} else {
		Err(Err::Error(Error::new("line", ErrorKind::Char)))
	}
}

fn parse_file_cmd(i: &str) -> IResult<&str, Command> {
	let (i, (c, s)) = tuple((anychar, opt(preceded(char(' '), parse_path))))(i)?;
	let cmd = match c {
		'e' => Command::Edit(s.map(ToString::to_string)),
		'f' => Command::File(
			s.ok_or(Err::Error(Error::new("line", ErrorKind::Char)))?
				.to_string(),
		),
		'r' => Command::Read(s.map(ToString::to_string)),
		'w' => Command::Write(s.map(ToString::to_string)),
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, cmd))
}

fn parse_exec_cmd(i: &str) -> IResult<&str, Command> {
	let (i, s) = preceded(char('!'), parse_path)(i)?;
	Ok((i, Command::Exec(s.to_string())))
}

fn parse_path(i: &str) -> IResult<&str, &str> {
	i.split_at_position1_complete(|item| is_newline(item as u8), ErrorKind::Fail)
}

// Insert Mode
pub fn parse_terminator(i: &str) -> IResult<&str, ()> {
	let (i, _) = terminated(char('.'), newline)(i)?;
	Ok((i, ()))
}

// Helpers
fn parse_address_range(i: &str) -> IResult<&str, AddressRange> {
	alt((
		parse_special_range,
		parse_tuple_range,
		parse_simple_range,
		parse_regex,
	))(i)
}

fn parse_regex(i: &str) -> IResult<&str, AddressRange> {
	let (i, (c, s, _)) = alt((
		tuple((char('/'), opt(many1(none_of("/?\n"))), opt(char('/')))),
		tuple((char('?'), opt(many1(none_of("/?\n"))), opt(char('?')))),
	))(i)?;
	match c {
		'/' => Ok((i, AddressRange::Next(s.map(|re| re.into_iter().collect())))),
		'?' => Ok((i, AddressRange::Prev(s.map(|re| re.into_iter().collect())))),
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	}
}

fn parse_special_range(i: &str) -> IResult<&str, AddressRange> {
	let (i, c) = anychar(i)?;
	let range = match c {
		'%' | ',' => AddressRange::Range(Address::Abs(0), Address::Abs(-1)),
		';' => AddressRange::Range(Address::Rel(0), Address::Abs(-1)),
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, range))
}

fn parse_tuple_range(i: &str) -> IResult<&str, AddressRange> {
	let (i, f) = parse_address(i)?;
	let (i, _) = char(',')(i)?;
	let (i, t) = parse_address(i)?;
	Ok((i, AddressRange::Range(f, t)))
}

fn parse_simple_range(i: &str) -> IResult<&str, AddressRange> {
	let (i, f) = parse_address(i)?;
	Ok((i, AddressRange::Range(f, f)))
}

fn parse_sign(i: &str) -> IResult<&str, char> {
	alt((char('+'), char('-')))(i)
}

fn parse_address(i: &str) -> IResult<&str, Address> {
	alt((parse_mark_addr, parse_line_addr, parse_special_addr))(i)
}

fn parse_special_addr(i: &str) -> IResult<&str, Address> {
	let (i, c) = anychar(i)?;
	let line = match c {
		'.' => Address::Rel(0),
		'$' => Address::Abs(-1),
		'+' => Address::Rel(1),
		'-' | '^' => Address::Rel(-1),
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, line))
}

fn parse_line_addr(i: &str) -> IResult<&str, Address> {
	let pref = parse_sign(i);
	let (i, o) = i32(i)?;
	match pref {
		Ok(_) => Ok((i, Address::Rel(o))),
		_ => {
			if o > 0 {
				Ok((i, Address::Abs(o - 1)))
			} else {
				return Err(Err::Error(Error::new("address", ErrorKind::Fail)));
			}
		}
	}
}

fn parse_mark_addr(i: &str) -> IResult<&str, Address> {
	let (i, c) = preceded(char('\''), anychar)(i)?;
	let c = c as u8;
	if c > 0x60 && c < 0x7b {
		Ok((i, Address::Mark(c - 0x61)))
	} else {
		Err(Err::Error(Error::new("address", ErrorKind::Fail)))
	}
}

// Print flags
fn parse_flag(i: &str) -> IResult<&str, PrintFlag> {
	let (i, c) = anychar(i)?;
	let f = match c {
		'n' => PrintFlag::Number,
		'p' => PrintFlag::Print,
		_ => return Err(Err::Error(Error::new("line", ErrorKind::Char))),
	};
	Ok((i, f))
}
