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

use std::fmt;
use std::iter::FromIterator;
use std::ops::RangeBounds;
use std::vec::Splice;

#[derive(Debug)]
pub struct Buffer {
	lines: Vec<String>,
}

impl Buffer {
	pub const fn new() -> Self {
		Buffer { lines: Vec::new() }
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.lines.len()
	}

	#[inline]
	pub fn push(&mut self, val: String) {
		self.lines.push(val)
	}

	#[inline]
	pub fn splice<R, I>(&mut self, range: R, replace_with: I) -> Splice<'_, I::IntoIter>
	where
		R: RangeBounds<usize>,
		I: IntoIterator<Item = String>,
	{
		self.lines.splice(range, replace_with)
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
		}
	}
}
