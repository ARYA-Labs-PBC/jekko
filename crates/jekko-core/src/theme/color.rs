//! [`Color`] struct, hex/ANSI parsing, and the tiny private hex decoder.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::ThemeError;

/// 8-bits-per-channel color with alpha.
///
/// Acts as the resolved-color representation for the theme system. Matches
/// the TypeScript runtime's hex-resolved color (the TS layer multiplies into
/// floating point internally; we keep the byte representation to avoid
/// floating-point drift in tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Color {
    /// Red channel (0-255).
    pub r: u8,
    /// Green channel (0-255).
    pub g: u8,
    /// Blue channel (0-255).
    pub b: u8,
    /// Alpha channel (0-255). Defaults to fully opaque.
    #[serde(default = "default_alpha")]
    pub a: u8,
}

fn default_alpha() -> u8 {
    255
}

impl Color {
    /// Opaque color from r/g/b bytes.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Fully transparent.
    pub const fn transparent() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }

    /// Parse a hex color (`#rgb`, `#rrggbb`, or `#rrggbbaa`).
    pub fn parse_hex(input: &str) -> Result<Self, ThemeError> {
        let stripped = input.strip_prefix('#').unwrap_or(input);
        let bytes = match stripped.len() {
            3 => {
                // expand #rgb -> #rrggbb
                let mut buf = String::with_capacity(6);
                for c in stripped.chars() {
                    buf.push(c);
                    buf.push(c);
                }
                hex::decode(&buf)
            }
            6 => hex::decode(stripped).map(|mut v| {
                v.push(255);
                v
            }),
            8 => hex::decode(stripped),
            _ => return Err(ThemeError::InvalidHex(input.to_string())),
        }
        .map_err(|_| ThemeError::InvalidHex(input.to_string()))?;
        Ok(Self {
            r: bytes[0],
            g: bytes[1],
            b: bytes[2],
            a: *bytes.get(3).unwrap_or(&255),
        })
    }

    /// Convert an ANSI 0-255 palette index to the canonical xterm color.
    pub fn from_ansi256(code: u8) -> Self {
        if code < 16 {
            const ANSI16: [(u8, u8, u8); 16] = [
                (0x00, 0x00, 0x00),
                (0x80, 0x00, 0x00),
                (0x00, 0x80, 0x00),
                (0x80, 0x80, 0x00),
                (0x00, 0x00, 0x80),
                (0x80, 0x00, 0x80),
                (0x00, 0x80, 0x80),
                (0xc0, 0xc0, 0xc0),
                (0x80, 0x80, 0x80),
                (0xff, 0x00, 0x00),
                (0x00, 0xff, 0x00),
                (0xff, 0xff, 0x00),
                (0x00, 0x00, 0xff),
                (0xff, 0x00, 0xff),
                (0x00, 0xff, 0xff),
                (0xff, 0xff, 0xff),
            ];
            let (r, g, b) = ANSI16[code as usize];
            return Color::rgb(r, g, b);
        }
        if code < 232 {
            let index = (code - 16) as u32;
            let b = index % 6;
            let g = (index / 6) % 6;
            let r = index / 36;
            let val = |x: u32| -> u8 {
                if x == 0 {
                    0
                } else {
                    (x * 40 + 55) as u8
                }
            };
            return Color::rgb(val(r), val(g), val(b));
        }
        let gray = (code - 232).saturating_mul(10).saturating_add(8);
        Color::rgb(gray, gray, gray)
    }
}

impl FromStr for Color {
    type Err = ThemeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_hex(s)
    }
}

mod hex {
    //! Tiny hex decoder. Avoids pulling in a third-party crate.
    pub fn decode(input: &str) -> Result<Vec<u8>, ()> {
        if !input.len().is_multiple_of(2) {
            return Err(());
        }
        let bytes = input.as_bytes();
        let mut out = Vec::with_capacity(bytes.len() / 2);
        for chunk in bytes.chunks(2) {
            let hi = nibble(chunk[0])?;
            let lo = nibble(chunk[1])?;
            out.push((hi << 4) | lo);
        }
        Ok(out)
    }

    fn nibble(c: u8) -> Result<u8, ()> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_6() {
        let c = Color::parse_hex("#11223344").unwrap();
        assert_eq!(
            c,
            Color {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 0x44
            }
        );
    }

    #[test]
    fn parse_hex_3() {
        let c = Color::parse_hex("#abc").unwrap();
        assert_eq!(c, Color::rgb(0xaa, 0xbb, 0xcc));
    }

    #[test]
    fn ansi256_basic() {
        assert_eq!(Color::from_ansi256(0), Color::rgb(0, 0, 0));
        assert_eq!(Color::from_ansi256(15), Color::rgb(0xff, 0xff, 0xff));
    }
}
