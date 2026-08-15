//! 极简 JSON 解析 / 序列化（零依赖，仅 std）
//! Copyright (C) 2026 MINO · Himer (MineACE)
//! SPDX-License-Identifier: Apache-2.0
//! 数值按原始字符串保存，避免浮点精度问题。
//! 本项目只用得到：对象 / 数组 / 字符串 / 布尔 / 整数。
//! 支持标准转义（\" \\ \/ \b \f \n \r \t \uXXXX）。

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    pub fn obj() -> Json {
        Json::Obj(BTreeMap::new())
    }

    pub fn as_obj(&self) -> Result<&BTreeMap<String, Json>, String> {
        match self {
            Json::Obj(m) => Ok(m),
            _ => Err("预期对象".to_string()),
        }
    }
    pub fn as_arr(&self) -> Result<&Vec<Json>, String> {
        match self {
            Json::Arr(a) => Ok(a),
            _ => Err("预期数组".to_string()),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(key),
            _ => None,
        }
    }
    pub fn get_str(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| match v {
            Json::Str(s) => Some(s.clone()),
            _ => None,
        })
    }
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| match v {
            Json::Bool(b) => Some(*b),
            _ => None,
        })
    }
    pub fn get_str_arr(&self, key: &str) -> Vec<String> {
        match self.get(key) {
            Some(Json::Arr(a)) => a
                .iter()
                .filter_map(|v| match v {
                    Json::Str(s) => Some(s.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

/* ---------------- 解析 ---------------- */

pub fn parse(input: &str) -> Result<Json, String> {
    let mut p = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(format!("JSON 末尾有额外内容（位置 {}）", p.pos));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }
    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
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
            Err(format!("位置 {}：预期字符 '{}'", self.pos, c as char))
        }
    }
    fn parse_value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Json::Str(self.parse_string()?)),
            Some(b't') => {
                self.expect_literal("true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal("false")?;
                Ok(Json::Bool(false))
            }
            Some(b'n') => {
                self.expect_literal("null")?;
                Ok(Json::Null)
            }
            Some(c) if c == b'-' || c.is_ascii_digit() => Ok(Json::Num(self.parse_number()?)),
            other => Err(format!(
                "位置 {}：非法字符 '{}'",
                self.pos,
                other.map(|c| c as char).unwrap_or(' ')
            )),
        }
    }
    fn expect_literal(&mut self, lit: &str) -> Result<(), String> {
        for (i, ch) in lit.bytes().enumerate() {
            if self.bytes.get(self.pos + i) != Some(&ch) {
                return Err(format!("位置 {}：预期关键字 {}", self.pos, lit));
            }
        }
        self.pos += lit.len();
        Ok(())
    }
    fn parse_object(&mut self) -> Result<Json, String> {
        self.expect(b'{')?;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Obj(map));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            let val = self.parse_value()?;
            map.insert(key, val);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("位置 {}：预期 , 或 }}", self.pos)),
            }
        }
        Ok(Json::Obj(map))
    }
    fn parse_array(&mut self) -> Result<Json, String> {
        self.expect(b'[')?;
        let mut arr = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Arr(arr));
        }
        loop {
            arr.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("位置 {}：预期 , 或 ]", self.pos)),
            }
        }
        Ok(Json::Arr(arr))
    }
    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or_else(|| "字符串未闭合".to_string())?;
            self.pos += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.peek().ok_or_else(|| "转义未完成".to_string())?;
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let code = self.parse_hex4()?;
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        other => return Err(format!("非法转义 \\{}", other as char)),
                    }
                }
                0..=0x1F => return Err("字符串含未转义控制字符".to_string()),
                _ => {
                    // 处理 UTF-8 多字节（按字节推进，但已自增 pos）
                    let start = self.pos - 1;
                    let mut end = self.pos;
                    while end < self.bytes.len() && (self.bytes[end] & 0xC0) == 0x80 {
                        end += 1;
                    }
                    let slice = &self.bytes[start..end];
                    out.push_str(&String::from_utf8_lossy(slice));
                    self.pos = end;
                }
            }
        }
        Ok(out)
    }
    fn parse_hex4(&mut self) -> Result<u32, String> {
        if self.pos + 4 > self.bytes.len() {
            return Err("\\u 转义不足 4 位".to_string());
        }
        let hex = &self.bytes[self.pos..self.pos + 4];
        self.pos += 4;
        let s = std::str::from_utf8(hex).map_err(|_| "非法 \\u".to_string())?;
        u32::from_str_radix(s, 16).map_err(|_| "非法 \\u".to_string())
    }
    fn parse_number(&mut self) -> Result<String, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == b'.' || c == b'e' || c == b'E' || c == b'+' || c == b'-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err("非法数字".to_string());
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned())
    }
}

/* ---------------- 序列化 ---------------- */

pub fn to_string(v: &Json) -> String {
    let mut s = String::new();
    write_value(v, &mut s, 0, false);
    s
}

pub fn to_string_pretty(v: &Json) -> String {
    let mut s = String::new();
    write_value(v, &mut s, 0, true);
    s
}

fn write_value(v: &Json, out: &mut String, indent: usize, pretty: bool) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(n) => out.push_str(n),
        Json::Str(s) => write_string(s, out),
        Json::Arr(a) => {
            if a.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if pretty {
                    out.push('\n');
                    out.push_str(&"  ".repeat(indent + 1));
                }
                write_value(item, out, indent + 1, pretty);
            }
            if pretty {
                out.push('\n');
                out.push_str(&"  ".repeat(indent));
            }
            out.push(']');
        }
        Json::Obj(m) => {
            if m.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (k, val)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if pretty {
                    out.push('\n');
                    out.push_str(&"  ".repeat(indent + 1));
                }
                write_string(k, out);
                out.push(':');
                if pretty {
                    out.push(' ');
                }
                write_value(val, out, indent + 1, pretty);
            }
            if pretty {
                out.push('\n');
                out.push_str(&"  ".repeat(indent));
            }
            out.push('}');
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
