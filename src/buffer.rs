/*
 * Copyright (c) 2022 Tobias Heider <me@tobhe.de>
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

use std::convert::TryFrom;
use std::fmt;
use std::iter::FromIterator;
use std::ops::Bound::{Excluded, Included, Unbounded};
use std::ops::RangeBounds;

#[derive(Debug)]
pub struct Buffer {
	pub marks: [Option<usize>; 26],
	pub changed: bool,

	lines: Vec<String>,
}

impl Buffer {
	pub const fn new() -> Self {
		Buffer {
			lines: Vec::new(),
			marks: [None; 26],
			changed: false,
		}
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.lines.len()
	}

	#[inline]
	pub fn push(&mut self, val: String) {
		self.lines.push(val)
	}

	pub fn replace_iter<R, I>(&mut self, range: R, replace_with: I)
	where
		R: RangeBounds<usize> + Clone,
		I: IntoIterator<Item = String>,
	{
		let old = self.lines.len() as i64;
		self.lines.splice(range.clone(), replace_with);
		let diff = old - (self.lines.len() as i64);

		// Remove marks in deleted range,
		for mark in self.marks.iter_mut() {
			*mark = if let Some(ref index) = mark {
				if range.contains(index) {
					None
				} else if range_after(&range, index) {
					Some(usize::try_from((*index as i64) - diff).unwrap())
				} else {
					Some(*index)
				}
			} else {
				*mark
			}
		}
		self.changed = true;
	}

	#[inline]
	pub fn iter(&self) -> std::slice::Iter<String> {
		self.lines.iter()
	}
}

impl Extend<String> for Buffer {
	#[inline]
	fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
		self.lines.extend(iter)
	}
}

impl fmt::Display for Buffer {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{}",
			self.lines.iter().fold(String::new(), |e, l| e + l + "\n")
		)
	}
}

impl IntoIterator for Buffer {
	type Item = String;
	type IntoIter = std::vec::IntoIter<String>;

	#[inline]
	fn into_iter(self) -> std::vec::IntoIter<String> {
		self.lines.into_iter()
	}
}

impl FromIterator<String> for Buffer {
	#[inline]
	fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Buffer {
		Buffer {
			lines: Vec::<String>::from_iter(iter.into_iter()),
			marks: [None; 26],
			changed: false,
		}
	}
}

fn range_after<R>(range: &R, item: &usize) -> bool
where
	R: RangeBounds<usize>,
{
	match range.end_bound() {
		Included(end) => item > end,
		Excluded(end) => item >= end,
		Unbounded => false,
	}
}
