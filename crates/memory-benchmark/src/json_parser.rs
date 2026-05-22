use crate::json::Json;
use std::collections::BTreeMap;

/// Very small JSON parser. Used only by prompt_reduce / population_report for
/// reading the lightweight JSONL ledger lines we produce ourselves. Supports a
/// subset: bool, null, int, float, string, array, object. No exponents on int.
pub fn parse(src: &str) -> Result<Json, String> {
    let mut p = Parser { src, pos: 0 };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos != src.len() {
        return Err(format!("extra input at byte {}", p.pos));
    }
    Ok(v)
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.src.as_bytes().get(self.pos).copied()
    }
    fn skip_ws(&mut self) {
        while let Some(b) = self.peek() {
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(format!("expected {:?} at byte {}", c as char, self.pos))
        }
    }
    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(Json::Str),
            Some(b't') | Some(b'f') => self.parse_bool(),
            Some(b'n') => self.parse_null(),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(c) => Err(format!("unexpected byte {:?} at {}", c as char, self.pos)),
            None => Err("unexpected EOF".to_string()),
        }
    }
    fn parse_null(&mut self) -> Result<Json, String> {
        if self.src[self.pos..].starts_with("null") {
            self.pos += 4;
            Ok(Json::Null)
        } else {
            Err(format!("invalid null at {}", self.pos))
        }
    }
    fn parse_bool(&mut self) -> Result<Json, String> {
        if self.src[self.pos..].starts_with("true") {
            self.pos += 4;
            Ok(Json::Bool(true))
        } else if self.src[self.pos..].starts_with("false") {
            self.pos += 5;
            Ok(Json::Bool(false))
        } else {
            Err(format!("invalid bool at {}", self.pos))
        }
    }
    fn parse_number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(b'0'..=b'9') = self.peek() {
            self.pos += 1;
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            while let Some(b'0'..=b'9') = self.peek() {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            while let Some(b'0'..=b'9') = self.peek() {
                self.pos += 1;
            }
        }
        let slice = &self.src[start..self.pos];
        if is_float {
            slice
                .parse::<f64>()
                .map(Json::Float)
                .map_err(|e| format!("bad number {}: {}", slice, e))
        } else {
            slice
                .parse::<i64>()
                .map(Json::Int)
                .map_err(|e| format!("bad int {}: {}", slice, e))
        }
    }
    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.bump_char() {
                Some('"') => return Ok(out),
                Some('\\') => match self.bump_char() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('b') => out.push('\u{08}'),
                    Some('f') => out.push('\u{0c}'),
                    Some('u') => out.push(self.parse_unicode_escape()?),
                    other => return Err(format!("bad escape {:?} at {}", other, self.pos)),
                },
                Some(c) => out.push(c),
                None => return Err("unterminated string".to_string()),
            }
        }
    }
    fn bump_char(&mut self) -> Option<char> {
        let ch = self.src[self.pos..].chars().next()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let high = self.parse_hex4()?;
        let codepoint = if (0xD800..=0xDBFF).contains(&high) {
            if !self.src[self.pos..].starts_with("\\u") {
                return Err("missing low surrogate in \\u escape".to_string());
            }
            self.pos += 2;
            let low = self.parse_hex4()?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err("invalid low surrogate in \\u escape".to_string());
            }
            0x10000 + (((high - 0xD800) << 10) | (low - 0xDC00))
        } else if (0xDC00..=0xDFFF).contains(&high) {
            return Err("unexpected low surrogate in \\u escape".to_string());
        } else {
            high
        };
        let Some(ch) = char::from_u32(codepoint) else {
            return Err(format!("invalid \\u escape codepoint: {codepoint:#x}"));
        };
        Ok(ch)
    }
    fn parse_hex4(&mut self) -> Result<u32, String> {
        let end = match self.pos.checked_add(4).filter(|end| *end <= self.src.len()) {
            Some(end) => end,
            None => return Err("truncated \\u escape".to_string()),
        };
        let hex = &self.src[self.pos..end];
        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("bad \\u escape at byte {}", self.pos));
        }
        self.pos = end;
        u32::from_str_radix(hex, 16).map_err(|err| format!("bad \\u escape: {err}"))
    }
    fn parse_array(&mut self) -> Result<Json, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    self.skip_ws();
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                Some(c) => {
                    return Err(format!(
                        "expected , or ] got {:?} at {}",
                        c as char, self.pos
                    ))
                }
                None => return Err("unterminated array".to_string()),
            }
        }
    }
    fn parse_object(&mut self) -> Result<Json, String> {
        self.expect(b'{')?;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(map));
        }
        loop {
            self.skip_ws();
            let k = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let v = self.parse_value()?;
            map.insert(k, v);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(map));
                }
                Some(c) => {
                    return Err(format!(
                        "expected , or }} got {:?} at {}",
                        c as char, self.pos
                    ))
                }
                None => return Err("unterminated object".to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::json::Json;

    #[test]
    fn parses_raw_utf8_and_unicode_escapes() {
        assert_eq!(
            parse(r#"{"text":"CPPD deposition (Figure 2 ). β"}"#).expect("json"),
            Json::Object(std::collections::BTreeMap::from([(
                "text".to_string(),
                Json::Str("CPPD deposition (Figure 2 ). β".to_string())
            )]))
        );
        assert_eq!(
            parse(r#"{"text":"snowman \u2603 smile \uD83D\uDE00"}"#).expect("json"),
            Json::Object(std::collections::BTreeMap::from([(
                "text".to_string(),
                Json::Str("snowman ☃ smile 😀".to_string())
            )]))
        );
    }
}
