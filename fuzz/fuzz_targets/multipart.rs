#![no_main]

use berserk::{Headers, Method, Request};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut headers = Headers::new();
    headers
        .append("content-type", "multipart/form-data; boundary=berserk")
        .expect("static content type is valid");
    let request = Request::new(
        Method::new("POST").expect("static HTTP method is valid"),
        "/upload",
        headers,
        data.to_vec(),
    )
    .expect("static request target is valid");
    let _ = request.multipart(16, 4_096);
});
