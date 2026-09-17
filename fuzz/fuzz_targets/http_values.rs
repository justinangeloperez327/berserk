#![no_main]

use berserk::{Headers, Method, Request};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let first = data.iter().position(|byte| *byte == 0).unwrap_or(data.len());
    let remainder = data.get(first.saturating_add(1)..).unwrap_or_default();
    let second = remainder
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(remainder.len());

    let Ok(target) = std::str::from_utf8(&data[..first]) else {
        return;
    };
    let Ok(name) = std::str::from_utf8(&remainder[..second]) else {
        return;
    };
    let Ok(value) = std::str::from_utf8(
        remainder
            .get(second.saturating_add(1)..)
            .unwrap_or_default(),
    ) else {
        return;
    };

    let mut headers = Headers::new();
    let _ = headers.append(name, value);
    let method = Method::new("GET").expect("static HTTP method is valid");
    let _ = Request::new(method, target, headers, Vec::new());
});
