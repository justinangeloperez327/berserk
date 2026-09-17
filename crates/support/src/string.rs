/// Laravel-style string helpers with Rust-native return types and semantics.
///
/// `Str` is intentionally stateless. Methods that can borrow from the input do so;
/// methods that transform text return an owned `String`.
#[derive(Clone, Copy, Debug, Default)]
pub struct Str;

impl Str {
    pub fn after<'a>(value: &'a str, search: &str) -> &'a str {
        if search.is_empty() {
            return value;
        }
        value
            .find(search)
            .map_or(value, |index| &value[index + search.len()..])
    }

    pub fn after_last<'a>(value: &'a str, search: &str) -> &'a str {
        if search.is_empty() {
            return value;
        }
        value
            .rfind(search)
            .map_or(value, |index| &value[index + search.len()..])
    }

    pub fn before<'a>(value: &'a str, search: &str) -> &'a str {
        if search.is_empty() {
            return value;
        }
        value.find(search).map_or(value, |index| &value[..index])
    }

    pub fn before_last<'a>(value: &'a str, search: &str) -> &'a str {
        if search.is_empty() {
            return value;
        }
        value.rfind(search).map_or(value, |index| &value[..index])
    }

    pub fn between<'a>(value: &'a str, from: &str, to: &str) -> &'a str {
        Self::before(Self::after(value, from), to)
    }

    pub fn between_first<'a>(value: &'a str, from: &str, to: &str) -> &'a str {
        if from.is_empty() || to.is_empty() {
            return value;
        }
        let Some(start) = value.find(from) else {
            return value;
        };
        let rest = &value[start + from.len()..];
        rest.find(to).map_or(value, |end| &rest[..end])
    }

    pub fn camel(value: &str) -> String {
        Self::lcfirst(&Self::studly(value))
    }

    pub fn char_at(value: &str, index: isize) -> Option<char> {
        let len = value.chars().count();
        let index = normalize_index(index, len)?;
        value.chars().nth(index)
    }

    pub fn chop_start(value: &str, prefix: &str) -> String {
        value.strip_prefix(prefix).unwrap_or(value).to_owned()
    }

    pub fn chop_end(value: &str, suffix: &str) -> String {
        value.strip_suffix(suffix).unwrap_or(value).to_owned()
    }

    pub fn contains(value: &str, needle: &str) -> bool {
        value.contains(needle)
    }

    pub fn contains_any<I, S>(value: &str, needles: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        needles.into_iter().any(|needle| value.contains(needle.as_ref()))
    }

    pub fn contains_all<I, S>(value: &str, needles: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        needles.into_iter().all(|needle| value.contains(needle.as_ref()))
    }

    pub fn doesnt_contain(value: &str, needle: &str) -> bool {
        !Self::contains(value, needle)
    }

    pub fn deduplicate(value: &str, character: char) -> String {
        let mut output = String::with_capacity(value.len());
        let mut previous_was_target = false;
        for current in value.chars() {
            if current == character {
                if !previous_was_target {
                    output.push(current);
                }
                previous_was_target = true;
            } else {
                output.push(current);
                previous_was_target = false;
            }
        }
        output
    }

    pub fn ends_with(value: &str, suffix: &str) -> bool {
        value.ends_with(suffix)
    }

    pub fn ends_with_any<I, S>(value: &str, suffixes: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        suffixes
            .into_iter()
            .any(|suffix| value.ends_with(suffix.as_ref()))
    }

    pub fn finish(value: &str, cap: &str) -> String {
        if cap.is_empty() {
            return value.to_owned();
        }
        format!("{}{}", value.trim_end_matches(cap), cap)
    }

    pub fn headline(value: &str) -> String {
        case_words(value)
            .into_iter()
            .map(|word| Self::ucfirst(&word.to_lowercase()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn is_ascii(value: &str) -> bool {
        value.is_ascii()
    }

    pub fn kebab(value: &str) -> String {
        case_words(value)
            .into_iter()
            .map(|word| word.to_lowercase())
            .collect::<Vec<_>>()
            .join("-")
    }

    pub fn lcfirst(value: &str) -> String {
        let mut chars = value.chars();
        let Some(first) = chars.next() else {
            return String::new();
        };
        let mut output = first.to_lowercase().collect::<String>();
        output.push_str(chars.as_str());
        output
    }

    pub fn length(value: &str) -> usize {
        value.chars().count()
    }

    pub fn limit(value: &str, limit: usize) -> String {
        Self::limit_with(value, limit, "...")
    }

    pub fn limit_with(value: &str, limit: usize, end: &str) -> String {
        if value.chars().count() <= limit {
            return value.to_owned();
        }
        let mut output = value.chars().take(limit).collect::<String>();
        output.push_str(end);
        output
    }

    pub fn lower(value: &str) -> String {
        value.to_lowercase()
    }

    pub fn mask(value: &str, mask: &str, index: isize, length: Option<usize>) -> String {
        let chars = value.chars().collect::<Vec<_>>();
        let total = chars.len();
        let start = if index < 0 {
            total.saturating_sub(index.unsigned_abs())
        } else {
            (index as usize).min(total)
        };
        let count = length.unwrap_or(total.saturating_sub(start));
        let end = start.saturating_add(count).min(total);
        let mut output = String::with_capacity(value.len());
        for (position, character) in chars.into_iter().enumerate() {
            if (start..end).contains(&position) {
                output.push_str(mask);
            } else {
                output.push(character);
            }
        }
        output
    }

    pub fn pad_left(value: &str, length: usize, pad: &str) -> String {
        let current = Self::length(value);
        if current >= length || pad.is_empty() {
            return value.to_owned();
        }
        format!("{}{}", pad_chars(pad, length - current), value)
    }

    pub fn pad_right(value: &str, length: usize, pad: &str) -> String {
        let current = Self::length(value);
        if current >= length || pad.is_empty() {
            return value.to_owned();
        }
        format!("{}{}", value, pad_chars(pad, length - current))
    }

    pub fn pad_both(value: &str, length: usize, pad: &str) -> String {
        let current = Self::length(value);
        if current >= length || pad.is_empty() {
            return value.to_owned();
        }
        let missing = length - current;
        let left = missing / 2;
        let right = missing - left;
        format!("{}{}{}", pad_chars(pad, left), value, pad_chars(pad, right))
    }

    pub fn position(value: &str, needle: &str) -> Option<usize> {
        value
            .find(needle)
            .map(|byte_index| value[..byte_index].chars().count())
    }

    pub fn remove(value: &str, search: &str) -> String {
        value.replace(search, "")
    }

    pub fn repeat(value: &str, times: usize) -> String {
        value.repeat(times)
    }

    pub fn replace(value: &str, search: &str, replacement: &str) -> String {
        value.replace(search, replacement)
    }

    pub fn replace_first(value: &str, search: &str, replacement: &str) -> String {
        value.replacen(search, replacement, 1)
    }

    pub fn replace_last(value: &str, search: &str, replacement: &str) -> String {
        if search.is_empty() {
            return value.to_owned();
        }
        let Some(index) = value.rfind(search) else {
            return value.to_owned();
        };
        let mut output = value.to_owned();
        output.replace_range(index..index + search.len(), replacement);
        output
    }

    pub fn reverse(value: &str) -> String {
        value.chars().rev().collect()
    }

    pub fn slug(value: &str) -> String {
        Self::slug_with(value, "-")
    }

    pub fn slug_with(value: &str, separator: &str) -> String {
        case_words(value)
            .into_iter()
            .map(|word| word.to_lowercase())
            .collect::<Vec<_>>()
            .join(separator)
    }

    pub fn snake(value: &str) -> String {
        case_words(value)
            .into_iter()
            .map(|word| word.to_lowercase())
            .collect::<Vec<_>>()
            .join("_")
    }

    pub fn squish(value: &str) -> String {
        value.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    pub fn start(value: &str, prefix: &str) -> String {
        if prefix.is_empty() {
            return value.to_owned();
        }
        format!("{}{}", prefix, value.trim_start_matches(prefix))
    }

    pub fn starts_with(value: &str, prefix: &str) -> bool {
        value.starts_with(prefix)
    }

    pub fn starts_with_any<I, S>(value: &str, prefixes: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        prefixes
            .into_iter()
            .any(|prefix| value.starts_with(prefix.as_ref()))
    }

    pub fn studly(value: &str) -> String {
        case_words(value)
            .into_iter()
            .map(|word| Self::ucfirst(&word.to_lowercase()))
            .collect()
    }

    pub fn substr(value: &str, start: isize, length: Option<usize>) -> String {
        let chars = value.chars().collect::<Vec<_>>();
        let total = chars.len();
        let start = if start < 0 {
            total.saturating_sub(start.unsigned_abs())
        } else {
            (start as usize).min(total)
        };
        let end = length
            .map(|length| start.saturating_add(length).min(total))
            .unwrap_or(total);
        chars[start..end].iter().collect()
    }

    pub fn substr_count(value: &str, needle: &str) -> usize {
        if needle.is_empty() {
            return 0;
        }
        value.matches(needle).count()
    }

    pub fn take(value: &str, limit: isize) -> String {
        if limit >= 0 {
            value.chars().take(limit as usize).collect()
        } else {
            let count = limit.unsigned_abs();
            let total = value.chars().count();
            value.chars().skip(total.saturating_sub(count)).collect()
        }
    }

    pub fn title(value: &str) -> String {
        value
            .split_whitespace()
            .map(|word| Self::ucfirst(&word.to_lowercase()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn trim(value: &str) -> String {
        value.trim().to_owned()
    }

    pub fn ltrim(value: &str) -> String {
        value.trim_start().to_owned()
    }

    pub fn rtrim(value: &str) -> String {
        value.trim_end().to_owned()
    }

    pub fn ucfirst(value: &str) -> String {
        let mut chars = value.chars();
        let Some(first) = chars.next() else {
            return String::new();
        };
        let mut output = first.to_uppercase().collect::<String>();
        output.push_str(chars.as_str());
        output
    }

    pub fn upper(value: &str) -> String {
        value.to_uppercase()
    }

    pub fn unwrap(value: &str, before: &str, after: &str) -> String {
        value
            .strip_prefix(before)
            .and_then(|value| value.strip_suffix(after))
            .unwrap_or(value)
            .to_owned()
    }

    pub fn word_count(value: &str) -> usize {
        value.split_whitespace().count()
    }

    pub fn words(value: &str, words: usize) -> String {
        Self::words_with(value, words, "...")
    }

    pub fn words_with(value: &str, words: usize, end: &str) -> String {
        let parts = value.split_whitespace().collect::<Vec<_>>();
        if parts.len() <= words {
            return parts.join(" ");
        }
        let mut output = parts[..words].join(" ");
        output.push_str(end);
        output
    }

    pub fn wrap(value: &str, before: &str, after: &str) -> String {
        format!("{before}{value}{after}")
    }
}

fn normalize_index(index: isize, len: usize) -> Option<usize> {
    if index >= 0 {
        let index = index as usize;
        (index < len).then_some(index)
    } else {
        let distance = index.unsigned_abs();
        (distance <= len).then_some(len - distance)
    }
}

fn pad_chars(pad: &str, length: usize) -> String {
    if length == 0 || pad.is_empty() {
        return String::new();
    }
    pad.chars().cycle().take(length).collect()
}

fn case_words(value: &str) -> Vec<String> {
    let chars = value.chars().collect::<Vec<_>>();
    let mut words = Vec::new();
    let mut current = String::new();

    for (index, character) in chars.iter().copied().enumerate() {
        if !character.is_alphanumeric() {
            push_word(&mut words, &mut current);
            continue;
        }

        if !current.is_empty() && character.is_uppercase() {
            let previous = chars.get(index.wrapping_sub(1)).copied();
            let next = chars.get(index + 1).copied();
            let follows_lower_or_digit = previous
                .is_some_and(|previous| previous.is_lowercase() || previous.is_numeric());
            let acronym_boundary = previous.is_some_and(char::is_uppercase)
                && next.is_some_and(char::is_lowercase);
            if follows_lower_or_digit || acronym_boundary {
                push_word(&mut words, &mut current);
            }
        }

        current.push(character);
    }

    push_word(&mut words, &mut current);
    words
}

fn push_word(words: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        words.push(std::mem::take(current));
    }
}

#[cfg(test)]
mod tests {
    use super::Str;

    #[test]
    fn slices_strings() {
        assert_eq!(Str::after("This is my name", "This is"), " my name");
        assert_eq!(Str::after_last("App\\Http\\Controllers", "\\"), "Controllers");
        assert_eq!(Str::before("This is my name", "my name"), "This is ");
        assert_eq!(Str::before_last("a/b/c", "/"), "a/b");
        assert_eq!(Str::between("[a] bc [d]", "[", "]"), "a");
        assert_eq!(Str::between_first("[a] bc [d]", "[", "]"), "a");
        assert_eq!(Str::substr("Berserk", 1, Some(3)), "ers");
        assert_eq!(Str::substr("Berserk", -3, None), "erk");
        assert_eq!(Str::take("Berserk", -3), "erk");
        assert_eq!(Str::char_at("Berserk", -1), Some('k'));
    }

    #[test]
    fn converts_case() {
        assert_eq!(Str::camel("foo_bar"), "fooBar");
        assert_eq!(Str::snake("XMLHttpRequest"), "xml_http_request");
        assert_eq!(Str::kebab("fooBar"), "foo-bar");
        assert_eq!(Str::studly("foo_bar"), "FooBar");
        assert_eq!(Str::headline("EmailNotificationSent"), "Email Notification Sent");
        assert_eq!(Str::title("berserk WEB framework"), "Berserk Web Framework");
        assert_eq!(Str::lower("BERSERK"), "berserk");
        assert_eq!(Str::upper("berserk"), "BERSERK");
        assert_eq!(Str::ucfirst("berserk"), "Berserk");
        assert_eq!(Str::lcfirst("Berserk"), "berserk");
    }

    #[test]
    fn normalizes_strings() {
        assert_eq!(Str::slug("Berserk Web Framework"), "berserk-web-framework");
        assert_eq!(Str::slug_with("Berserk Web Framework", "_"), "berserk_web_framework");
        assert_eq!(Str::squish("  berserk   web\n framework  "), "berserk web framework");
        assert_eq!(Str::start("api/users", "/"), "/api/users");
        assert_eq!(Str::finish("api/users///", "/"), "api/users/");
        assert_eq!(Str::deduplicate("a---b", '-'), "a-b");
    }

    #[test]
    fn searches_and_replaces() {
        assert!(Str::contains("Berserk Framework", "Framework"));
        assert!(Str::contains_any("Berserk Framework", ["Laravel", "Berserk"]));
        assert!(Str::contains_all("Berserk Framework", ["Berserk", "Framework"]));
        assert!(Str::doesnt_contain("Berserk", "Laravel"));
        assert!(Str::starts_with("Berserk", "Ber"));
        assert!(Str::ends_with("Berserk", "erk"));
        assert_eq!(Str::position("Café Berserk", "Berserk"), Some(5));
        assert_eq!(Str::replace_first("foo foo", "foo", "bar"), "bar foo");
        assert_eq!(Str::replace_last("foo foo", "foo", "bar"), "foo bar");
        assert_eq!(Str::remove("Berserk Framework", " Framework"), "Berserk");
    }

    #[test]
    fn formats_strings() {
        assert_eq!(Str::limit("Berserk Framework", 7), "Berserk...");
        assert_eq!(Str::words("Berserk is a Rust framework", 3), "Berserk is a...");
        assert_eq!(Str::pad_left("7", 3, "0"), "007");
        assert_eq!(Str::pad_right("7", 3, "0"), "700");
        assert_eq!(Str::pad_both("7", 3, "0"), "070");
        assert_eq!(Str::mask("123456789", "*", 2, Some(4)), "12****789");
        assert_eq!(Str::reverse("Berserk"), "kresreB");
        assert_eq!(Str::repeat("ab", 3), "ababab");
        assert_eq!(Str::wrap("Berserk", "[", "]"), "[Berserk]");
        assert_eq!(Str::unwrap("[Berserk]", "[", "]"), "Berserk");
    }

    #[test]
    fn measures_strings_by_unicode_scalar_values() {
        assert_eq!(Str::length("Café"), 4);
        assert_eq!(Str::word_count("Berserk web framework"), 3);
        assert_eq!(Str::substr_count("one two one", "one"), 2);
        assert!(Str::is_ascii("Berserk"));
        assert!(!Str::is_ascii("Café"));
    }
}
