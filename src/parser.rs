extern crate nom;

use nom::{
	Err,
	error::Error,
	error::ErrorKind,
	IResult,
	branch::alt,
	sequence::tuple,
	Parser,
	character::complete::char,
	character::complete::i32,
	character::complete::newline,
};

#[derive(Debug)]
pub enum Line {
	Abs(i32),
	Rel(i32),
}

#[derive(Debug)]
pub struct Range {
	from: Line,
	to: Line,
}

/*
 * Commands: All commands are single characters, some require additional
 * parameters.
 */
#[derive(Debug)]
pub enum Command {
	Append(Line),	// (.)a		Append text to the buffer
	Change(Range),	// (.,.)c	Change line in buffer
	Edit(String),	// e file	Edit file
	EditU(String),	// E file	Edit file uncoditionally
	FName(String),	// f file	Set default filename to file
	Goto(Line),	// n		Go to line
	Insert(Line),	// (.)i		Insert text before current line
	Read(Line),	// ($)r		Reads file to after the addressed line
	Number(Range),	// (.,.)n	Print lines with index
	Print(Range),	// (.,.)p	Print lines
	Quit		// q		Quit
}

pub fn parse_command(i: &str) -> IResult<&str, Command> {
	alt((
		parse_append,
		parse_change,
		parse_goto,
		parse_insert,
		parse_number,
		parse_print,
	))(i)
}

// Commands
fn parse_append(i: &str) -> IResult<&str, Command> {
	let (i, l) = parse_line(i)?;
	let (i, _) = char('a')(i)?;
	let (i, _) = newline(i)?;
	Ok((i, Command::Append(l)))
}

fn parse_change(i: &str) -> IResult<&str, Command> {
	let (i, r) = parse_range(i)?;
	let (i, _) = char('c')(i)?;
	let (i, _) = newline(i)?;
	Ok((i, Command::Change(r)))
}

fn parse_goto(i: &str) -> IResult<&str, Command> {
	let (i, l) = parse_line(i)?;
	let (i, _) = newline(i)?;
	Ok((i, Command::Goto(l)))
}

fn parse_insert(i: &str) -> IResult<&str, Command> {
	let (i, l) = parse_line(i)?;
	let (i, _) = char('i')(i)?;
	let (i, _) = newline(i)?;
	Ok((i, Command::Insert(l)))
}

fn parse_number(i: &str) -> IResult<&str, Command> {
	let (i, r) = parse_range(i)?;
	let (i, _) = char('n')(i)?;
	let (i, _) = newline(i)?;
	Ok((i, Command::Number(r)))
}

fn parse_print(i: &str) -> IResult<&str, Command> {
	let (i, r) = parse_range(i)?;
	let (i, _) = char('p')(i)?;
	let (i, _) = newline(i)?;
	Ok((i, Command::Print(r)))
}

fn parse_read(i: &str) -> IResult<&str, Command> {
	let (i, l) = parse_line(i)?;
	let (i, _) = char('r')(i)?;
	let (i, _) = newline(i)?;
	Ok((i, Command::Read(l)))
}

// Helpers
fn parse_range(i: &str) -> IResult<&str, Range> {
	alt((
		parse_range_special,
		parse_range_tuple,
	))(i)
}

fn parse_range_special(i: &str) -> IResult<&str, Range> {
	match alt((char(','), char('%')))(i) {
		Ok((i, _)) => Ok((i, Range{from: Line::Abs(0),
		    to: Line::Abs(-1)})),
		Err(error) => Err(error),
	}
}

fn parse_sign(i: &str) -> IResult<&str, char> {
	alt((char('+'), char('-')))(i)
}

fn parse_line(i: &str) -> IResult<&str, Line> {
	alt((parse_line_regular, parse_line_special))(i)
}

fn parse_line_special(i: &str) -> IResult<&str, Line> {
	match alt((char('.'), char('$'), char('+'), char('-')))(i) {
		Ok((i, '.')) => Ok((i, Line::Rel(0))),
		Ok((i, '$')) => Ok((i, Line::Abs(-1))),
		Ok((i, '+')) => Ok((i, Line::Rel(1))),
		Ok((i, '-')) => Ok((i, Line::Rel(-1))),
		Err(e) => Err(e),
		// This should not happen but silences the compiler
		_ => Err(Err::Error(Error::new(" line", ErrorKind::Fail)))
	}
}

fn parse_line_regular(i: &str) -> IResult<&str, Line> {
	let pref = parse_sign(i);
	let (i, o) = i32(i)?;
	match pref {
		Ok(_) => Ok((i, Line::Rel(o))),
		_ => Ok((i, Line::Abs(o)))
	}
}

fn parse_range_tuple(i: &str) -> IResult<&str, Range> {
	// XXX:	addresses:	. $ - + %
	//	ranges:		, ;
	let (i, f) = parse_line(i)?;
	let (i, _) = char(',')(i)?;
	let (i, t) = parse_line(i)?;
	let r = Range{from: f, to: t};
	Ok((i, r))
}

