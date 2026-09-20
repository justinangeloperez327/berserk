use crate::{Middleware, Next, Request, Response, Result};
use rand_core::{OsRng, RngCore};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceContext {
    trace_id: String,
    parent_id: String,
    flags: String,
}

impl TraceContext {
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<_> = value.split('-').collect();
        if parts.len() != 4
            || parts[0] != "00"
            || !valid_hex(parts[1], 32)
            || !valid_hex(parts[2], 16)
            || !valid_hex(parts[3], 2)
        {
            return None;
        }
        if parts[1].bytes().all(|byte| byte == b'0') || parts[2].bytes().all(|byte| byte == b'0') {
            return None;
        }
        Some(Self {
            trace_id: parts[1].to_ascii_lowercase(),
            parent_id: parts[2].to_ascii_lowercase(),
            flags: parts[3].to_ascii_lowercase(),
        })
    }
    pub fn generate() -> Self {
        Self {
            trace_id: random_hex::<16>(),
            parent_id: random_hex::<8>(),
            flags: "01".into(),
        }
    }
    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            parent_id: random_hex::<8>(),
            flags: self.flags.clone(),
        }
    }
    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }
    pub fn parent_id(&self) -> &str {
        &self.parent_id
    }
    pub fn flags(&self) -> &str {
        &self.flags
    }
    pub fn sampled(&self) -> bool {
        u8::from_str_radix(&self.flags, 16)
            .map(|flags| flags & 1 == 1)
            .unwrap_or(false)
    }
    pub fn traceparent(&self) -> String {
        format!("00-{}-{}-{}", self.trace_id, self.parent_id, self.flags)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TraceLayer;
impl Middleware for TraceLayer {
    fn handle(&self, mut request: Request, next: Next<'_>) -> Result<Response> {
        let traceparents: Vec<_> = request.headers().get_all("traceparent").collect();
        let context = (traceparents.len() == 1)
            .then(|| traceparents[0])
            .and_then(TraceContext::parse)
            .map(|parent| parent.child())
            .unwrap_or_else(TraceContext::generate);
        let header = context.traceparent();
        request.set_trace_context(context);
        next.run(request)?.header("traceparent", &header)
    }
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn random_hex<const N: usize>() -> String {
    let mut bytes = [0_u8; N];
    loop {
        OsRng.fill_bytes(&mut bytes);
        if bytes.iter().any(|byte| *byte != 0) {
            return hex(&bytes);
        }
    }
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 15) as usize] as char);
    }
    output
}
