#![no_std]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
	Lower,
	Upper,
}

// this makes compilation slower
const fn gen_lut(c: Case) -> [u8; 0x10] {
	let α = match c {
		Case::Lower => b'a',
		Case::Upper => b'A',
	};
	let mut h = [0u8; 0x10];
	let mut i: u8 = 0;
	while i < 0x10 {
		h[i as usize] = i + if i < 10 { b'0' } else { α - 10 };
		i += 1;
	}
	h
}
pub const LUT_LOW: [u8; 0x10] = gen_lut(Case::Lower);
pub const LUT_UP: [u8; 0x10] = gen_lut(Case::Upper);

#[must_use]
pub const fn from_byte(b: u8, c: Case) -> [u8; 2] {
	let lut = match c {
		Case::Lower => LUT_LOW,
		Case::Upper => LUT_UP,
	};
	let mut h = [0, 0];
	h[0] = lut[(b >> 4) as usize];
	h[1] = lut[(b & 0xf) as usize];
	h
}

#[must_use]
const fn from_nibble(h: u8) -> u8 {
	match h {
		b'A'..=b'Z' => h - b'A' + 10,
		b'a'..=b'z' => h - b'a' + 10,
		b'0'..=b'9' => h - b'0',
		_ => 0xff, // Err
	}
}

/// # Errors
/// If any of the two bytes is not an ASCII nibble
pub const fn from_hex(mut h: [u8; 2]) -> Result<u8, ()> {
	// reuse memory
	h[0] = from_nibble(h[0]);
	h[1] = from_nibble(h[1]);
	// we could split this in 2 `if`s,
	// to avoid calling `from_nibble`,
	// but that might be slower
	if h[0] > 0xf || h[1] > 0xf {
		return Err(());
	}
	Ok((h[0] << 4) | h[1])
}

// iterator adapters aren't provided, as they're trivial:
// `iterable.into_iter().map(from_*)`

/// Expands the bytes from `src` into `dest[0..src.len() * 2]`.
///
/// If `dest` has spare bytes at the end,
/// `unsafe` code can assume they're intact.
///
/// See [`encode_slice_in_place`] if you need "zero-copy".
///
/// # Errors
/// If `src.len() > dest.len() / 2`
pub const fn encode_slice<'i, 'o>(src: &'i [u8], dest: &'o mut [u8], c: Case) -> Result<(), ()> {
	if src.len() > dest.len() / 2 {
		return Err(());
	}
	// from this point, there should be no bounds-checks
	// on indexing
	let mut i = 0;
	while i < src.len() {
		// never overflow
		let i2 = i + i;
		let [h0, h1] = from_byte(src[i], c);
		// err: `(&mut dest[i2..=(i2 | 1)]) = from_byte(src[i]);`
		// not `const`: `dest[i2..=(i2 | 1)].copy_from_slice(&from_byte(src[i]));`
		dest[i2] = h0;
		dest[i2 | 1] = h1;
		i += 1;
	}
	Ok(())
}

/// Expands the bytes from `s[0..s.len() / 2]` into `s[s.len() % 2..]`.
///
/// If `s.len()` is odd, `unsafe` code can assume the 1st byte is intact.
///
/// See [`encode_slice`] if you're not tight on memory.
pub const fn encode_slice_in_place(s: &mut [u8], c: Case) {
	let mut i = s.len() / 2;
	// so this is how we write `rev` `for`-loops
	// as `const`? huh, interesting...
	while let Some(j) = i.checked_sub(1) {
		i = j;
		// never overflow
		let i2 = i + i;
		let [h0, h1] = from_byte(s[i], c);
		s[i2] = h0;
		s[i2 | 1] = h1;
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeHexError {
	Odd,
	NotNibble,
	Small,
}
impl core::fmt::Display for DecodeHexError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Self::Odd => "Buffer has an odd `len`",
				Self::NotNibble => "One or more bytes is not an ASCII nibble",
				Self::Small => "Destination buffer isn't big enough",
			}
		)
	}
}
impl core::error::Error for DecodeHexError {}

/// Packs the nibbles from `src` into `dest`.
///
/// # Errors
/// Early-returns if `s.len()` is odd
pub const fn decode_slice<'i, 'o>(src: &'i [u8], dest: &'o mut [u8]) -> Result<(), DecodeHexError> {
	if !src.len().is_multiple_of(2) {
		return Err(DecodeHexError::Odd);
	}
	if src.len() / 2 > dest.len() {
		return Err(DecodeHexError::Small);
	}
	let len = src.len() / 2;
	let mut i = 0;
	while i < len {
		// never overflow
		let i2 = i + i;
		match from_hex([src[i2], src[i2 | 1]]) {
			Ok(b) => dest[i] = b,
			_ => return Err(DecodeHexError::NotNibble),
		}
		i += 1;
	}
	Ok(())
}

/// Packs the nibbles from `s[0..]` into `s[0..(s.len() / 2)]`.
///
/// `unsafe` code can assume the 2nd half is intact.
///
/// If `s.len()` is odd, `unsafe` code can assume the 1st byte is intact.
///
/// # Errors
/// Early-returns if `s.len()` is odd
pub const fn decode_slice_in_place(s: &mut [u8]) -> Result<(), DecodeHexError> {
	if !s.len().is_multiple_of(2) {
		return Err(DecodeHexError::Odd);
	}
	let len = s.len() / 2;
	let mut i = 0;
	while i < len {
		// never overflow
		let i2 = i + i;
		match from_hex([s[i2], s[i2 | 1]]) {
			Ok(b) => s[i] = b,
			_ => return Err(DecodeHexError::NotNibble),
		}
		i += 1;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sanity() {
		assert_eq!(&LUT_LOW, b"0123456789abcdef");
		assert_eq!(&LUT_UP, b"0123456789ABCDEF");
	}

	#[test]
	fn slice_works() {
		let mut dest = [0; 10];

		encode_slice_in_place(&mut dest, Case::Upper);
		assert_eq!(dest, *b"0000000000");

		encode_slice(b"hello", &mut dest, Case::Lower).unwrap();
		assert_eq!(dest, *b"68656c6c6f");

		decode_slice_in_place(&mut dest).unwrap();
		assert_eq!(dest, *b"helloc6c6f");
	}
}
