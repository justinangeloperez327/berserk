use std::fmt::{self, Write};

pub(crate) fn html_into(out: &mut impl Write, value: &str) -> fmt::Result {
    let mut start = 0;
    for (index, character) in value.char_indices() {
        let escaped = match character {
            '&' => "&amp;",
            '<' => "&lt;",
            '>' => "&gt;",
            '"' => "&quot;",
            '\'' => "&#39;",
            _ => continue,
        };
        out.write_str(&value[start..index])?;
        out.write_str(escaped)?;
        start = index + character.len_utf8();
    }
    out.write_str(&value[start..])
}
