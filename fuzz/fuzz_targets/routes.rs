#![no_main]

use berserk::{App, Response};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(path) = std::str::from_utf8(data) else {
        return;
    };
    let mut app = App::new();
    let _ = app.route().get(path, Response::empty);
});
