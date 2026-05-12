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
    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
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
            match self.bump() {
                Some(b'"') => return Ok(out),
                Some(b'\\') => match self.bump() {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'n') => out.push('\n'),
                    Some(b'r') => out.push('\r'),
                    Some(b't') => out.push('\t'),
                    Some(b'b') => out.push('\u{08}'),
                    Some(b'f') => out.push('\u{0c}'),
                    Some(b'u') => {
                        let hex = &self.src[self.pos..self.pos + 4];
                        let cp = u32::from_str_radix(hex, 16)
                            .map_err(|e| format!("bad \\u escape: {}", e))?;
                        if let Some(c) = char::from_u32(cp) {
                            out.push(c);
                        }
                        self.pos += 4;
                    }
                    other => {
                        return Err(format!(
                            "bad escape {:?} at {}",
                            other.map(|b| b as char),
                            self.pos
                        ))
                    }
                },
                Some(c) => out.push(c as char),
                None => return Err("unterminated string".to_string()),
            }
        }
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
