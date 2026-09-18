use berserk::{Headers, Json, Method, Request, Response};
#[test]
fn syntax_unicode_and_roundtrip() {
    let valid = br#"{"text":"\ud83d\ude00","number":-12.50e+3,"array":[true,false,null]}"#;
    let v = Json::parse(valid).unwrap();
    assert_eq!(v, Json::parse(v.encode().unwrap().as_bytes()).unwrap());
    assert_eq!(v.get("text").unwrap().as_str(), Some("😀"));
    for bad in [
        "01",
        "-",
        "1.",
        "1e",
        "NaN",
        "[1,]",
        "{\"a\":1,\"a\":2}",
        "\"\\ud800\"",
        "true false",
    ] {
        assert!(Json::parse(bad.as_bytes()).is_err(), "{bad}");
    }
    assert!(Json::parse(format!("{}0{}", "[".repeat(66), "]".repeat(66)).as_bytes()).is_err());
}
#[test]
fn request_content_type_query_and_response() {
    let mut h = Headers::new();
    h.insert("content-type", "application/json").unwrap();
    let req = Request::new(
        Method::new("POST").unwrap(),
        "/?q=hello+world&q=%E2%9C%93",
        h,
        b"{}".to_vec(),
    )
    .unwrap();
    assert!(req.json::<Json>().is_ok());
    assert_eq!(
        req.query_pairs().unwrap(),
        vec![("q".into(), "hello world".into()), ("q".into(), "✓".into())]
    );
    let no_type = Request::new(
        Method::new("POST").unwrap(),
        "/",
        Headers::new(),
        b"{}".to_vec(),
    )
    .unwrap();
    assert!(no_type.json::<Json>().is_err());
    assert_eq!(
        Response::json(&Json::Null)
            .unwrap()
            .headers()
            .get("content-type"),
        Some("application/json")
    );
}
#[test]
fn validation_accumulates() {
    let mut e = berserk::ValidationErrors::default();
    e.required("name", Some(" "));
    e.range("age", -1, 0, 120);
    assert_eq!(e.finish().unwrap_err().0.len(), 2);
}
