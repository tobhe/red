extern crate nom;

use nom::{
	Err,
	error::Error,
	error::ErrorKind,
	IResult,
	branch::alt,
	combinator::opt,
	sequence::tuple,
	Parser,
	character::complete::char,
	character::complete::i32,
	character::complete::newline,
};

#[derive(Clone, Copy, Debug)]
pub enum Line {
	Rel(i32),
	Abs(i32),
}

#[derive(Clone, Copy, Debug)]
pub struct Range {
	pub from: Line,
	pub to: Line,
}

impl Default for Range {
    fn default() -> Self {Range {from: Line::Rel(0), to: Line::Rel(0)}}
}

/*
 * Commands: All commands are single characters, some require additional
 * parameters.
 */
#[derive(Debug)]
pub enum Command {
	Append,		// (.)a		Append text to the buffer
	Change,		// (.,.)c	Change line in buffer
	Edit(String),	// e file	Edit file
	EditU(String),	// E file	Edit file uncoditionally
	FName(String),	// f file	Set default filename to file
	Goto,		// n		Go to line
	Insert,		// (.)i		Insert text before current line
	Read,		// ($)r		Reads file to after the addressed line
	Number,		// (.,.)n	Print lines with index
	Print,		// (.,.)p	Print lines
	Prompt,		// P		Enable * prompt
	Quit		// q		Quit
}

pub fn parse_command(i: &str) -> IResult<&str, (Range, Option<Command>)> {
	let (i, (r, c)) = (tuple((
	    opt(parse_range),
	    opt(alt((
		parse_append,
		parse_change,
		parse_insert,
		parse_number,
		parse_print,
		parse_prompt,
		parse_read,
	    ))),
	))(i))?;
	let (i, _) = newline(i)?;
	Ok((i, (r.unwrap_or_default(), c)))
}

// Commands
fn parse_append(i: &str) -> IResult<&str, Command> {
	let (i, _) = char('a')(i)?;
	Ok((i, Command::Append))
}

fn parse_change(i: &str) -> IResult<&str, Command> {
	let (i, _) = char('c')(i)?;
	Ok((i, Command::Change))
}

fn parse_insert(i: &str) -> IResult<&str, Command> {
	let (i, _) = char('i')(i)?;
	Ok((i, Command::Insert))
}

fn parse_number(i: &str) -> IResult<&str, Command> {
	let (i, _) = char('n')(i)?;
	Ok((i, Command::Number))
}

fn parse_print(i: &str) -> IResult<&str, Command> {
	let (i, _) = char('p')(i)?;
	Ok((i, Command::Print))
}

fn parse_prompt(i: &str) -> IResult<&str, Command> {
	let (i, _) = char('P')(i)?;
	Ok((i, Command::Prompt))
}

fn parse_read(i: &str) -> IResult<&str, Command> {
	let (i, _) = char('r')(i)?;
	Ok((i, Command::Read))
}

// Helpers
fn parse_range(i: &str) -> IResult<&str, Range> {
	alt((
		parse_range_special,
		parse_range_tuple,
		parse_range_simple,
	))(i)
}

fn parse_range_special(i: &str) -> IResult<&str, Range> {
	match alt((char(','), char('%')))(i) {
		Ok((i, _)) => Ok((i, Range{from: Line::Abs(0),
		    to: Line::Abs(-1)})),
		Err(error) => Err(error),
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

fn parse_range_simple(i: &str) -> IResult<&str, Range> {
	let (i, f) = parse_line(i)?;
	let r = Range{from: f, to: f};
	Ok((i, r))
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
		_ => Ok((i, Line::Abs(o - 1)))
	}
}
