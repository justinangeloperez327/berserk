#![no_main]

use berserk::Json;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = Json::parse(data) {
        let encoded = value.encode().expect("parsed JSON must remain encodable");
        let reparsed = Json::parse(encoded.as_bytes()).expect("encoded JSON must parse");
        assert_eq!(value, reparsed);
    }
});
