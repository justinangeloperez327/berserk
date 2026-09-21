use berserk_axe::{Context, Error, SafeHtml, Template, Value};

#[test]
fn html_is_preserved_and_values_are_escaped() {
    let template = Template::compile("<h1>{{ title }}</h1>").unwrap();
    let context = Context::new().with("title", "<Axe & Berserk>");
    assert_eq!(
        template.render(&context).unwrap(),
        "<h1>&lt;Axe &amp; Berserk&gt;</h1>"
    );
}

#[test]
fn conditions_and_loops_render_nested_values() {
    let template = Template::compile(
        "@if(show)<ul>@foreach(user in users)<li>{{ user.name }}</li>@endforeach</ul>@else<p>Hidden</p>@endif",
    )
    .unwrap();
    let users = vec![
        Value::object([("name", Value::from("Ada"))]),
        Value::object([("name", Value::from("Linus"))]),
    ];
    let context = Context::new()
        .with("show", true)
        .with("users", Value::List(users));
    assert_eq!(
        template.render(&context).unwrap(),
        "<ul><li>Ada</li><li>Linus</li></ul>"
    );
}

#[test]
fn false_condition_uses_else_branch() {
    let template = Template::compile("@if(show)yes@elseno@endif").unwrap();
    let context = Context::new().with("show", false);
    assert_eq!(template.render(&context).unwrap(), "no");
}

#[test]
fn raw_output_requires_safe_html() {
    let template = Template::compile("{!! content !!}").unwrap();
    let unsafe_context = Context::new().with("content", "<strong>unsafe</strong>");
    assert!(matches!(
        template.render(&unsafe_context),
        Err(Error::UnsafeRaw(path)) if path == "content"
    ));

    let safe_context = Context::new().with("content", SafeHtml::new("<strong>safe</strong>"));
    assert_eq!(
        template.render(&safe_context).unwrap(),
        "<strong>safe</strong>"
    );
}

#[test]
fn missing_values_are_reported() {
    let template = Template::compile("{{ missing }}").unwrap();
    assert!(matches!(
        template.render(&Context::new()),
        Err(Error::MissingValue(path)) if path == "missing"
    ));
}

#[test]
fn malformed_templates_fail_during_compile() {
    assert_eq!(
        Template::compile("@foreach(user users)x@endforeach").unwrap_err(),
        Error::InvalidDirective("@foreach(user users)".into())
    );
    assert_eq!(
        Template::compile("@if(show)x").unwrap_err(),
        Error::UnclosedDirective("@if")
    );
}
