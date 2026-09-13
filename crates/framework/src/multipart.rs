//! Buffered multipart/form-data subset. No filesystem writes or filename trust.
use crate::{Headers, Request};
pub struct Part {
    pub headers: Headers,
    pub body: Vec<u8>,
}
impl std::fmt::Debug for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Part")
            .field("header_count", &self.headers.len())
            .field("body_bytes", &self.body.len())
            .finish()
    }
}
#[derive(Debug, Clone, Copy)]
pub struct MultipartError;
impl std::fmt::Display for MultipartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid or oversized multipart body")
    }
}
impl std::error::Error for MultipartError {}
impl Request {
    pub fn multipart(
        &self,
        max_parts: usize,
        max_part_headers: usize,
    ) -> Result<Vec<Part>, MultipartError> {
        let types: Vec<_> = self.headers().get_all("content-type").collect();
        if types.len() != 1 {
            return Err(MultipartError);
        }
        let mut fields = types[0].split(';');
        if !fields
            .next()
            .unwrap_or("")
            .trim()
            .eq_ignore_ascii_case("multipart/form-data")
        {
            return Err(MultipartError);
        }
        let mut boundary = None;
        for field in fields {
            let (key, value) = field.trim().split_once('=').ok_or(MultipartError)?;
            if !key.eq_ignore_ascii_case("boundary") || boundary.is_some() {
                return Err(MultipartError);
            }
            let value = value.trim();
            let value = if value.starts_with('"') {
                value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .ok_or(MultipartError)?
            } else {
                value
            };
            if value.is_empty()
                || value.len() > 70
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"'()+_,-./:=?".contains(&b))
            {
                return Err(MultipartError);
            }
            boundary = Some(value);
        }
        let marker = format!("--{}", boundary.ok_or(MultipartError)?).into_bytes();
        let data = self.body();
        let mut pos = 0;
        let mut parts = Vec::new();
        loop {
            if !data[pos..].starts_with(&marker) {
                return Err(MultipartError);
            }
            pos += marker.len();
            if data[pos..].starts_with(b"--") {
                pos += 2;
                if data[pos..].starts_with(b"\r\n") {
                    pos += 2;
                }
                return if pos == data.len() {
                    Ok(parts)
                } else {
                    Err(MultipartError)
                };
            }
            if !data[pos..].starts_with(b"\r\n") || parts.len() >= max_parts {
                return Err(MultipartError);
            }
            pos += 2;
            let end = data[pos..]
                .windows(4)
                .position(|v| v == b"\r\n\r\n")
                .ok_or(MultipartError)?;
            if end > max_part_headers {
                return Err(MultipartError);
            }
            let text = std::str::from_utf8(&data[pos..pos + end]).map_err(|_| MultipartError)?;
            let mut headers = Headers::new();
            for line in text.split("\r\n") {
                let (k, v) = line.split_once(':').ok_or(MultipartError)?;
                headers.append(k, v).map_err(|_| MultipartError)?;
            }
            let dispositions: Vec<_> = headers.get_all("content-disposition").collect();
            if dispositions.len() != 1
                || !dispositions[0]
                    .split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .eq_ignore_ascii_case("form-data")
            {
                return Err(MultipartError);
            }
            pos += end + 4;
            let mut delimiter = b"\r\n".to_vec();
            delimiter.extend_from_slice(&marker);
            let end = data[pos..]
                .windows(delimiter.len())
                .enumerate()
                .find_map(|(i, w)| {
                    let tail = &data[pos + i + delimiter.len()..];
                    if w == delimiter && (tail.starts_with(b"\r\n") || tail.starts_with(b"--")) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .ok_or(MultipartError)?;
            parts.push(Part {
                headers,
                body: data[pos..pos + end].to_vec(),
            });
            pos += end + 2;
        }
    }
}
