mod parser;

extern crate nom;

use crate::parser::{
	parse_command,
	Command,
	Line
};
use std::fs;
use std::env;
use std::io;
use std::convert::TryFrom;

struct State {
	line: u32,
	total: u32,
	mode: Mode,
}

enum Mode {
	CommandMode,
	InsertMode,
}

fn update_line(s: &mut State, l: Line) {
	match l {
		Line::Abs(c) => {
			if c < 0 {
				s.line = u32::try_from(i64::from(s.line)
				    - i64::from(c)).unwrap();
			} else {
				s.line = u32::try_from(c).unwrap();
			}
		}
		Line::Rel(c) => {}
	};
}

fn handle_command(s: &mut State, c: Command) {
	println!("success {:?}", c);
	match c {
		Command::Goto(l) => update_line(s, l),
		_ => {}
	};
}

fn main() {
	let mut state = State {line: 0, total: 0, mode: Mode::CommandMode};
	let stdin = io::stdin();

	loop {
		let mut input = String::new();
		match io::stdin().read_line(&mut input) {
			Ok(n) => {	
				match parse_command(&input) {
					Ok((input, n)) => {
						handle_command(&mut state, n);
					}
					Err(_) => println!("?"),
				}
			}
			Err(error) => {
			    println!("error: {}", error);
			}
		}
	}
}
