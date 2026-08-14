//! Northstar RFC 8785 JCS profile. No `unsafe`.
//! Keys sort by unsigned UTF-16 code units. Lone surrogates fail closed.

use crate::error::{Error, Result};
use std::collections::BTreeSet;

pub const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;
pub const MIN_SAFE_INTEGER: i64 = -9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

/// JCS text produced only after a value passes the Northstar profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Canonical(String);

impl Canonical {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Canonical {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Value {
    pub fn int(n: i64) -> Result<Self> {
        let v = Value::Int(n);
        v.validate()?;
        Ok(v)
    }

    pub fn object(pairs: Vec<(String, Value)>) -> Result<Self> {
        let v = Value::Object(pairs);
        v.validate()?;
        Ok(v)
    }

    pub fn validate(&self) -> Result<()> {
        match self {
            Value::Int(n) => {
                if *n > MAX_SAFE_INTEGER || *n < MIN_SAFE_INTEGER {
                    return Err(Error::jcs(format!("integer out of safe range: {n}")));
                }
                Ok(())
            }
            Value::Array(items) => {
                for item in items {
                    item.validate()?;
                }
                Ok(())
            }
            Value::Object(pairs) => {
                let mut seen = BTreeSet::new();
                for (k, v) in pairs {
                    if !seen.insert(k.as_str()) {
                        return Err(Error::jcs(format!("duplicate key {:?}", k)));
                    }
                    v.validate()?;
                }
                Ok(())
            }
            Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
        }
    }
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.i += ch.len_utf8();
        Some(ch)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\n' | '\r' | '\t')) {
            self.bump();
        }
    }

    fn starts_with(&self, lit: &str) -> bool {
        self.s[self.i..].starts_with(lit)
    }

    fn eat(&mut self, lit: &str) -> bool {
        if self.starts_with(lit) {
            self.i += lit.len();
            true
        } else {
            false
        }
    }
}

pub fn parse(text: &str) -> Result<Value> {
    let mut p = Parser { s: text, i: 0 };
    let v = parse_value(&mut p)?;
    p.skip_ws();
    if p.i != p.s.len() {
        return Err(Error::jcs("trailing data"));
    }
    Ok(v)
}

fn parse_value(p: &mut Parser<'_>) -> Result<Value> {
    p.skip_ws();
    match p.peek() {
        None => Err(Error::jcs("unexpected end")),
        Some('{') => parse_object(p),
        Some('[') => parse_array(p),
        Some('"') => Ok(Value::String(parse_string(p)?)),
        Some('t' | 'f' | 'n') => parse_literal(p),
        Some('-' | '0'..='9') => parse_number(p),
        Some(c) => Err(Error::jcs(format!("unexpected character {c:?}"))),
    }
}

fn parse_literal(p: &mut Parser<'_>) -> Result<Value> {
    if p.eat("true") {
        Ok(Value::Bool(true))
    } else if p.eat("false") {
        Ok(Value::Bool(false))
    } else if p.eat("null") {
        Ok(Value::Null)
    } else {
        Err(Error::jcs("invalid literal"))
    }
}

fn parse_number(p: &mut Parser<'_>) -> Result<Value> {
    let start = p.i;
    if p.peek() == Some('-') {
        p.bump();
    }
    match p.peek() {
        Some('0') => {
            p.bump();
        }
        Some('1'..='9') => {
            while matches!(p.peek(), Some('0'..='9')) {
                p.bump();
            }
        }
        _ => return Err(Error::jcs("invalid number")),
    }
    if let Some('.' | 'e' | 'E') = p.peek() {
        return Err(Error::jcs("floating-point values are prohibited"));
    }
    let token = &p.s[start..p.i];
    if token == "-0" {
        return Err(Error::jcs("negative zero is prohibited"));
    }
    let n: i64 = token
        .parse()
        .map_err(|_| Error::jcs(format!("invalid integer {token}")))?;
    if !(MIN_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&n) {
        return Err(Error::jcs(format!("integer out of safe range: {token}")));
    }
    Ok(Value::Int(n))
}

fn parse_hex4(p: &mut Parser<'_>) -> Result<u16> {
    let rest = &p.s[p.i..];
    if rest.len() < 4 || !rest.as_bytes()[..4].iter().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::jcs("bad unicode escape"));
    }
    let unit = u16::from_str_radix(&rest[..4], 16).map_err(|_| Error::jcs("bad unicode escape"))?;
    p.i += 4;
    Ok(unit)
}

fn append_pair(out: &mut String, high: u16, low: u16) -> Result<()> {
    let cp = 0x10000 + (((high as u32) - 0xD800) << 10) + ((low as u32) - 0xDC00);
    let ch = char::from_u32(cp).ok_or_else(|| Error::jcs("lone surrogate"))?;
    out.push(ch);
    Ok(())
}

fn parse_string(p: &mut Parser<'_>) -> Result<String> {
    if p.bump() != Some('"') {
        return Err(Error::jcs("expected string"));
    }
    let mut out = String::new();
    loop {
        match p.peek() {
            None => return Err(Error::jcs("unterminated string")),
            Some('"') => {
                p.bump();
                return Ok(out);
            }
            Some('\\') => {
                p.bump();
                match p.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{0008}'),
                    Some('f') => out.push('\u{000c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let unit = parse_hex4(p)?;
                        if (0xD800..=0xDBFF).contains(&unit) {
                            if !p.eat("\\u") {
                                return Err(Error::jcs("lone surrogate"));
                            }
                            let low = parse_hex4(p)?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err(Error::jcs("lone surrogate"));
                            }
                            append_pair(&mut out, unit, low)?;
                        } else if (0xDC00..=0xDFFF).contains(&unit) {
                            return Err(Error::jcs("lone surrogate"));
                        } else {
                            out.push(
                                char::from_u32(unit as u32)
                                    .ok_or_else(|| Error::jcs("bad unicode escape"))?,
                            );
                        }
                    }
                    _ => return Err(Error::jcs("bad escape")),
                }
            }
            Some(ch) => {
                let cu = ch as u32;
                if cu < 0x20 {
                    return Err(Error::jcs("unescaped control character"));
                }
                // Valid UTF-8 cannot contain lone surrogates; reject if we ever see them.
                if (0xD800..=0xDFFF).contains(&cu) {
                    return Err(Error::jcs("lone surrogate"));
                }
                p.bump();
                out.push(ch);
            }
        }
    }
}

fn parse_object(p: &mut Parser<'_>) -> Result<Value> {
    p.bump(); // {
    p.skip_ws();
    if p.peek() == Some('}') {
        p.bump();
        return Ok(Value::Object(Vec::new()));
    }
    let mut pairs = Vec::new();
    let mut seen = BTreeSet::new();
    loop {
        p.skip_ws();
        if p.peek() != Some('"') {
            return Err(Error::jcs("expected object key"));
        }
        let key = parse_string(p)?;
        if !seen.insert(key.clone()) {
            return Err(Error::jcs(format!("duplicate key {:?}", key)));
        }
        p.skip_ws();
        if p.bump() != Some(':') {
            return Err(Error::jcs("expected :"));
        }
        let val = parse_value(p)?;
        pairs.push((key, val));
        p.skip_ws();
        match p.peek() {
            Some(',') => {
                p.bump();
            }
            Some('}') => {
                p.bump();
                return Ok(Value::Object(pairs));
            }
            _ => return Err(Error::jcs("expected , or }")),
        }
    }
}

fn parse_array(p: &mut Parser<'_>) -> Result<Value> {
    p.bump(); // [
    p.skip_ws();
    if p.peek() == Some(']') {
        p.bump();
        return Ok(Value::Array(Vec::new()));
    }
    let mut items = Vec::new();
    loop {
        items.push(parse_value(p)?);
        p.skip_ws();
        match p.peek() {
            Some(',') => {
                p.bump();
            }
            Some(']') => {
                p.bump();
                return Ok(Value::Array(items));
            }
            _ => return Err(Error::jcs("expected , or ]")),
        }
    }
}

fn cmp_utf16(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

fn encode_string(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{000c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn write_canonical(value: &Value) -> Result<String> {
    Ok(match value {
        Value::Null => "null".into(),
        Value::Bool(true) => "true".into(),
        Value::Bool(false) => "false".into(),
        Value::Int(n) => n.to_string(),
        Value::String(s) => encode_string(s),
        Value::Array(items) => {
            let mut inner = Vec::with_capacity(items.len());
            for item in items {
                inner.push(write_canonical(item)?);
            }
            format!("[{}]", inner.join(","))
        }
        Value::Object(pairs) => {
            let mut keys: Vec<&(String, Value)> = pairs.iter().collect();
            keys.sort_by(|a, b| cmp_utf16(&a.0, &b.0));
            let mut inner = Vec::with_capacity(keys.len());
            for (k, v) in keys {
                inner.push(format!("{}:{}", encode_string(k), write_canonical(v)?));
            }
            format!("{{{}}}", inner.join(","))
        }
    })
}

pub fn canonicalize(value: &Value) -> Result<Canonical> {
    value.validate()?;
    Ok(Canonical(write_canonical(value)?))
}

pub fn canonicalize_json_text(text: &str) -> Result<Canonical> {
    canonicalize(&parse(text)?)
}

pub fn utf8_hex(canonical: &str) -> String {
    canonical
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
