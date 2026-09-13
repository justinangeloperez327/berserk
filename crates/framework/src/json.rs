//! Strict JSON syntax, preserved numeric text and bounded nesting.
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Number(String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
}
impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid JSON at byte {}", self.offset)
    }
}
impl std::error::Error for JsonError {}
impl Number {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn as_i64(&self) -> Option<i64> {
        self.0.parse().ok()
    }
}
impl From<i64> for Json {
    fn from(n: i64) -> Self {
        Self::Number(Number(n.to_string()))
    }
}
impl From<u64> for Json {
    fn from(n: u64) -> Self {
        Self::Number(Number(n.to_string()))
    }
}
impl From<u128> for Json {
    fn from(n: u128) -> Self {
        Self::Number(Number(n.to_string()))
    }
}
impl From<bool> for Json {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}
impl From<String> for Json {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&str> for Json {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}
impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Self::Object(v) => v.get(key),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, JsonError> {
        std::str::from_utf8(bytes).map_err(|e| JsonError {
            offset: e.valid_up_to(),
        })?;
        let mut p = Parser { b: bytes, i: 0 };
        let value = p.value(0)?;
        p.ws();
        if p.i != bytes.len() {
            return p.err();
        }
        Ok(value)
    }
    pub fn encode(&self) -> Result<String, JsonError> {
        let mut out = String::new();
        self.write(&mut out, 0)?;
        Ok(out)
    }
    fn write(&self, out: &mut String, depth: usize) -> Result<(), JsonError> {
        if depth > 64 {
            return Err(JsonError { offset: out.len() });
        }
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Self::Number(n) => out.push_str(n.as_str()),
            Self::String(s) => quote(s, out),
            Self::Array(values) => {
                out.push('[');
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out, depth + 1)?;
                }
                out.push(']');
            }
            Self::Object(values) => {
                out.push('{');
                for (i, (k, v)) in values.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    quote(k, out);
                    out.push(':');
                    v.write(out, depth + 1)?;
                }
                out.push('}');
            }
        }
        Ok(())
    }
}
fn quote(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}
impl Parser<'_> {
    fn err<T>(&self) -> Result<T, JsonError> {
        Err(JsonError { offset: self.i })
    }
    fn ws(&mut self) {
        while self.b.get(self.i).is_some_and(|b| b" \t\r\n".contains(b)) {
            self.i += 1;
        }
    }
    fn eat(&mut self, b: u8) -> bool {
        if self.b.get(self.i) == Some(&b) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn hex(&mut self) -> Result<u32, JsonError> {
        let mut n = 0;
        for _ in 0..4 {
            let Some(b) = self.b.get(self.i) else {
                return self.err();
            };
            let Some(d) = (*b as char).to_digit(16) else {
                return self.err();
            };
            self.i += 1;
            n = n * 16 + d;
        }
        Ok(n)
    }
    fn string(&mut self) -> Result<String, JsonError> {
        if !self.eat(b'"') {
            return self.err();
        }
        let mut out = String::new();
        loop {
            let Some(&b) = self.b.get(self.i) else {
                return self.err();
            };
            if b == b'"' {
                self.i += 1;
                return Ok(out);
            }
            if b < 32 {
                return self.err();
            }
            if b == b'\\' {
                self.i += 1;
                let Some(&e) = self.b.get(self.i) else {
                    return self.err();
                };
                self.i += 1;
                match e {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let mut n = self.hex()?;
                        if (0xd800..=0xdbff).contains(&n) {
                            if !self.eat(b'\\') || !self.eat(b'u') {
                                return self.err();
                            }
                            let low = self.hex()?;
                            if !(0xdc00..=0xdfff).contains(&low) {
                                return self.err();
                            }
                            n = 0x10000 + ((n - 0xd800) << 10) + (low - 0xdc00);
                        }
                        let Some(c) = char::from_u32(n) else {
                            return self.err();
                        };
                        out.push(c);
                    }
                    _ => return self.err(),
                }
            } else {
                let tail = std::str::from_utf8(&self.b[self.i..])
                    .map_err(|_| JsonError { offset: self.i })?;
                let c = tail.chars().next().ok_or(JsonError { offset: self.i })?;
                self.i += c.len_utf8();
                out.push(c);
            }
        }
    }
    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.ws();
        if depth > 64 {
            return self.err();
        }
        match self.b.get(self.i).copied() {
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                self.ws();
                let mut v = Vec::new();
                if self.eat(b']') {
                    return Ok(Json::Array(v));
                }
                loop {
                    v.push(self.value(depth + 1)?);
                    self.ws();
                    if self.eat(b']') {
                        break;
                    }
                    if !self.eat(b',') {
                        return self.err();
                    }
                }
                Ok(Json::Array(v))
            }
            Some(b'{') => {
                self.i += 1;
                self.ws();
                let mut v = BTreeMap::new();
                if self.eat(b'}') {
                    return Ok(Json::Object(v));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.ws();
                    if !self.eat(b':') {
                        return self.err();
                    }
                    let value = self.value(depth + 1)?;
                    if v.insert(k, value).is_some() {
                        return self.err();
                    }
                    self.ws();
                    if self.eat(b'}') {
                        break;
                    }
                    if !self.eat(b',') {
                        return self.err();
                    }
                }
                Ok(Json::Object(v))
            }
            Some(b'n' | b't' | b'f') => {
                for (s, v) in [
                    ("null", Json::Null),
                    ("true", Json::Bool(true)),
                    ("false", Json::Bool(false)),
                ] {
                    if self.b[self.i..].starts_with(s.as_bytes()) {
                        self.i += s.len();
                        return Ok(v);
                    }
                }
                self.err()
            }
            Some(b'-' | b'0'..=b'9') => {
                let start = self.i;
                self.eat(b'-');
                if !self.eat(b'0') {
                    let before = self.i;
                    while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                        self.i += 1;
                    }
                    if self.i == before {
                        return self.err();
                    }
                }
                if self.eat(b'.') {
                    let before = self.i;
                    while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                        self.i += 1;
                    }
                    if self.i == before {
                        return self.err();
                    }
                }
                if self.eat(b'e') || self.eat(b'E') {
                    if !self.eat(b'+') {
                        self.eat(b'-');
                    }
                    let before = self.i;
                    while self.b.get(self.i).is_some_and(u8::is_ascii_digit) {
                        self.i += 1;
                    }
                    if self.i == before {
                        return self.err();
                    }
                }
                Ok(Json::Number(Number(
                    std::str::from_utf8(&self.b[start..self.i]).unwrap().into(),
                )))
            }
            _ => self.err(),
        }
    }
}
