use crate::{Middleware, Next, Request, Response, Result};

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
    /// Generate a trace context. Prefer `try_generate` in server code.
    ///
    /// # Panics
    /// Panics if operating-system randomness is unavailable.
    pub fn generate() -> Self {
        Self::try_generate().expect("trace randomness unavailable")
    }
    pub fn try_generate() -> Result<Self> {
        Ok(Self {
            trace_id: random_hex::<16>()?,
            parent_id: random_hex::<8>()?,
            flags: "01".into(),
        })
    }
    /// Generate a child context. Prefer `try_child` in server code.
    ///
    /// # Panics
    /// Panics if operating-system randomness is unavailable.
    pub fn child(&self) -> Self {
        self.try_child().expect("trace randomness unavailable")
    }
    pub fn try_child(&self) -> Result<Self> {
        Ok(Self {
            trace_id: self.trace_id.clone(),
            parent_id: random_hex::<8>()?,
            flags: self.flags.clone(),
        })
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
        let parent = (traceparents.len() == 1)
            .then(|| traceparents[0])
            .and_then(TraceContext::parse);
        let context = match parent {
            Some(parent) => parent.try_child()?,
            None => TraceContext::try_generate()?,
        };
        let header = context.traceparent();
        request.set_trace_context(context);
        next.run(request)?.header("traceparent", &header)
    }
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn random_hex<const N: usize>() -> Result<String> {
    random_hex_with::<N>(|bytes| {
        getrandom::fill(bytes).map_err(|_| {
            super::OperationalError::new("operating system randomness unavailable").into()
        })
    })
}
fn random_hex_with<const N: usize>(
    mut fill: impl FnMut(&mut [u8]) -> Result<()>,
) -> Result<String> {
    let mut bytes = [0_u8; N];
    for _ in 0..8 {
        fill(&mut bytes)?;
        if bytes.iter().any(|byte| *byte != 0) {
            return Ok(hex(&bytes));
        }
    }
    Err(super::OperationalError::new("could not generate a nonzero trace identifier").into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_randomness_errors_propagate_without_panicking() {
        let error = random_hex_with::<16>(|_| {
            Err(super::super::OperationalError::new("entropy unavailable").into())
        });
        assert!(error.is_err());
    }

    #[test]
    fn zero_trace_identifiers_have_a_bounded_retry_budget() {
        let mut attempts = 0;
        assert!(random_hex_with::<8>(|bytes| {
            attempts += 1;
            bytes.fill(0);
            Ok(())
        })
        .is_err());
        assert_eq!(attempts, 8);
    }

    #[test]
    fn fallible_contexts_round_trip_and_preserve_parent_trace() {
        let parent = TraceContext::try_generate().unwrap();
        let child = parent.try_child().unwrap();
        assert_eq!(parent.trace_id(), child.trace_id());
        assert_eq!(TraceContext::parse(&child.traceparent()), Some(child));
    }
}
