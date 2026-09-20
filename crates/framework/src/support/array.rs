use crate::Json;
use rand_core::{OsRng, RngCore};
use std::{collections::BTreeMap, fmt::Display};

/// Collection and nested-value helpers for Rust collections and Berserk `Json`.
///
/// Typed collection helpers operate on `Vec` / `BTreeMap`. Dot-path helpers
/// operate on `Json`, Berserk's dynamic nested value representation. These
/// helpers are explicit utilities, not framework-global facades.
#[derive(Clone, Copy, Debug, Default)]
pub struct Arr;

impl Arr {
    pub fn collapse<T, I, J>(arrays: I) -> Vec<T>
    where
        I: IntoIterator<Item = J>,
        J: IntoIterator<Item = T>,
    {
        arrays.into_iter().flatten().collect()
    }

    pub fn cross_join<T: Clone>(arrays: &[Vec<T>]) -> Vec<Vec<T>> {
        let mut rows = vec![Vec::new()];
        for values in arrays {
            if values.is_empty() {
                return Vec::new();
            }
            let mut next = Vec::with_capacity(rows.len().saturating_mul(values.len()));
            for row in rows {
                for value in values {
                    let mut expanded = row.clone();
                    expanded.push(value.clone());
                    next.push(expanded);
                }
            }
            rows = next;
        }
        rows
    }

    pub fn divide<K: Clone + Ord, V: Clone>(map: &BTreeMap<K, V>) -> (Vec<K>, Vec<V>) {
        (
            map.keys().cloned().collect(),
            map.values().cloned().collect(),
        )
    }

    pub fn exists<V>(map: &BTreeMap<String, V>, key: &str) -> bool {
        map.contains_key(key)
    }

    pub fn except<V: Clone, I, S>(map: &BTreeMap<String, V>, keys: I) -> BTreeMap<String, V>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let excluded = keys
            .into_iter()
            .map(|key| key.as_ref().to_owned())
            .collect::<Vec<_>>();
        map.iter()
            .filter(|(key, _)| !excluded.iter().any(|candidate| candidate == *key))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    pub fn only<V: Clone, I, S>(map: &BTreeMap<String, V>, keys: I) -> BTreeMap<String, V>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        keys.into_iter()
            .filter_map(|key| {
                let key = key.as_ref();
                map.get(key).map(|value| (key.to_owned(), value.clone()))
            })
            .collect()
    }

    pub fn prepend_keys_with<V>(map: BTreeMap<String, V>, prefix: &str) -> BTreeMap<String, V> {
        map.into_iter()
            .map(|(key, value)| (format!("{prefix}{key}"), value))
            .collect()
    }

    pub fn first<T>(values: &[T]) -> Option<&T> {
        values.first()
    }

    pub fn first_where<T>(
        values: &[T],
        mut predicate: impl FnMut(&T, usize) -> bool,
    ) -> Option<&T> {
        values
            .iter()
            .enumerate()
            .find(|(index, value)| predicate(value, *index))
            .map(|(_, value)| value)
    }

    pub fn last<T>(values: &[T]) -> Option<&T> {
        values.last()
    }

    pub fn last_where<T>(values: &[T], mut predicate: impl FnMut(&T, usize) -> bool) -> Option<&T> {
        values
            .iter()
            .enumerate()
            .rev()
            .find(|(index, value)| predicate(value, *index))
            .map(|(_, value)| value)
    }

    pub fn take<T: Clone>(values: &[T], limit: isize) -> Vec<T> {
        if limit >= 0 {
            values.iter().take(limit as usize).cloned().collect()
        } else {
            let count = limit.unsigned_abs().min(values.len());
            values[values.len() - count..].to_vec()
        }
    }

    pub fn join<T: Display>(values: &[T], glue: &str, final_glue: &str) -> String {
        match values {
            [] => String::new(),
            [only] => only.to_string(),
            _ => {
                let separator = if final_glue.is_empty() {
                    glue
                } else {
                    final_glue
                };
                let mut output = values[..values.len() - 1]
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(glue);
                output.push_str(separator);
                output.push_str(&values[values.len() - 1].to_string());
                output
            }
        }
    }

    pub fn key_by<T, K, I>(values: I, mut key: impl FnMut(&T) -> K) -> BTreeMap<K, T>
    where
        I: IntoIterator<Item = T>,
        K: Ord,
    {
        values
            .into_iter()
            .map(|value| (key(&value), value))
            .collect()
    }

    pub fn map<T, U>(values: &[T], mut callback: impl FnMut(&T, usize) -> U) -> Vec<U> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| callback(value, index))
            .collect()
    }

    pub fn map_with_keys<T, K, V, I>(
        values: I,
        mut callback: impl FnMut(T) -> (K, V),
    ) -> BTreeMap<K, V>
    where
        I: IntoIterator<Item = T>,
        K: Ord,
    {
        values.into_iter().map(&mut callback).collect()
    }

    pub fn prepend<T>(mut values: Vec<T>, value: T) -> Vec<T> {
        values.insert(0, value);
        values
    }

    pub fn random<T>(values: &[T]) -> Option<&T> {
        (!values.is_empty()).then(|| &values[random_index(values.len())])
    }

    pub fn random_multiple<T: Clone>(values: &[T], count: usize) -> Vec<T> {
        let mut shuffled = Self::shuffle(values);
        shuffled.truncate(count.min(shuffled.len()));
        shuffled
    }

    pub fn shuffle<T: Clone>(values: &[T]) -> Vec<T> {
        let mut shuffled = values.to_vec();
        if shuffled.len() < 2 {
            return shuffled;
        }
        for index in (1..shuffled.len()).rev() {
            let target = random_index(index + 1);
            shuffled.swap(index, target);
        }
        shuffled
    }

    pub fn sort<T: Ord>(mut values: Vec<T>) -> Vec<T> {
        values.sort();
        values
    }

    pub fn sort_desc<T: Ord>(mut values: Vec<T>) -> Vec<T> {
        values.sort_by(|left, right| right.cmp(left));
        values
    }

    pub fn every<T>(values: &[T], mut predicate: impl FnMut(&T, usize) -> bool) -> bool {
        values
            .iter()
            .enumerate()
            .all(|(index, value)| predicate(value, index))
    }

    pub fn some<T>(values: &[T], mut predicate: impl FnMut(&T, usize) -> bool) -> bool {
        values
            .iter()
            .enumerate()
            .any(|(index, value)| predicate(value, index))
    }

    pub fn where_<T>(values: &[T], mut predicate: impl FnMut(&T, usize) -> bool) -> Vec<&T> {
        values
            .iter()
            .enumerate()
            .filter(|(index, value)| predicate(value, *index))
            .map(|(_, value)| value)
            .collect()
    }

    pub fn reject<T>(values: &[T], mut predicate: impl FnMut(&T, usize) -> bool) -> Vec<&T> {
        values
            .iter()
            .enumerate()
            .filter(|(index, value)| !predicate(value, *index))
            .map(|(_, value)| value)
            .collect()
    }

    pub fn partition<T>(
        values: &[T],
        mut predicate: impl FnMut(&T, usize) -> bool,
    ) -> (Vec<&T>, Vec<&T>) {
        let mut matches = Vec::new();
        let mut rejected = Vec::new();
        for (index, value) in values.iter().enumerate() {
            if predicate(value, index) {
                matches.push(value);
            } else {
                rejected.push(value);
            }
        }
        (matches, rejected)
    }

    pub fn where_not_null<T>(values: &[Option<T>]) -> Vec<&T> {
        values.iter().filter_map(Option::as_ref).collect()
    }

    pub fn wrap<T>(value: Option<T>) -> Vec<T> {
        value.into_iter().collect()
    }

    pub fn get<'a>(value: &'a Json, path: &str) -> Option<&'a Json> {
        if path.is_empty() {
            return Some(value);
        }
        let mut current = value;
        for segment in path.split('.') {
            current = match current {
                Json::Object(values) => values.get(segment)?,
                Json::Array(values) => values.get(segment.parse::<usize>().ok()?)?,
                _ => return None,
            };
        }
        Some(current)
    }

    pub fn has(value: &Json, path: &str) -> bool {
        Self::get(value, path).is_some()
    }

    pub fn has_all<I, S>(value: &Json, paths: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        paths
            .into_iter()
            .all(|path| Self::has(value, path.as_ref()))
    }

    pub fn has_any<I, S>(value: &Json, paths: I) -> bool
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        paths
            .into_iter()
            .any(|path| Self::has(value, path.as_ref()))
    }

    pub fn add(value: &mut Json, path: &str, item: Json) {
        if Self::get(value, path).is_none_or(|existing| matches!(existing, Json::Null)) {
            Self::set(value, path, item);
        }
    }

    pub fn set(value: &mut Json, path: &str, item: Json) {
        if path.is_empty() {
            *value = item;
            return;
        }
        let segments = path.split('.').collect::<Vec<_>>();
        set_path(value, &segments, item);
    }

    pub fn forget(value: &mut Json, path: &str) -> bool {
        if path.is_empty() {
            *value = Json::Null;
            return true;
        }
        let segments = path.split('.').collect::<Vec<_>>();
        forget_path(value, &segments)
    }

    pub fn pull(value: &mut Json, path: &str) -> Option<Json> {
        let pulled = Self::get(value, path)?.clone();
        Self::forget(value, path);
        Some(pulled)
    }

    pub fn push(value: &mut Json, path: &str, item: Json) {
        if !Self::has(value, path) {
            Self::set(value, path, Json::Array(Vec::new()));
        }
        let Some(target) = get_mut(value, path) else {
            return;
        };
        match target {
            Json::Array(values) => values.push(item),
            _ => *target = Json::Array(vec![item]),
        }
    }

    pub fn dot(value: &Json) -> BTreeMap<String, Json> {
        let mut output = BTreeMap::new();
        dot_into(value, "", &mut output);
        output
    }

    pub fn undot(values: BTreeMap<String, Json>) -> Json {
        let mut output = Json::Object(BTreeMap::new());
        for (path, value) in values {
            Self::set(&mut output, &path, value);
        }
        output
    }

    pub fn flatten(value: &Json) -> Vec<Json> {
        let mut output = Vec::new();
        flatten_into(value, &mut output);
        output
    }

    pub fn pluck(values: &[Json], path: &str) -> Vec<Json> {
        values
            .iter()
            .filter_map(|value| Self::get(value, path).cloned())
            .collect()
    }

    pub fn array<'a>(value: &'a Json, path: &str) -> Option<&'a [Json]> {
        match Self::get(value, path)? {
            Json::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn string<'a>(value: &'a Json, path: &str) -> Option<&'a str> {
        match Self::get(value, path)? {
            Json::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn integer(value: &Json, path: &str) -> Option<i64> {
        match Self::get(value, path)? {
            Json::Number(value) => value.as_i64(),
            _ => None,
        }
    }

    pub fn float(value: &Json, path: &str) -> Option<f64> {
        match Self::get(value, path)? {
            Json::Number(value) => value.as_str().parse().ok(),
            _ => None,
        }
    }

    pub fn boolean(value: &Json, path: &str) -> Option<bool> {
        match Self::get(value, path)? {
            Json::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

fn random_index(len: usize) -> usize {
    let bound = u64::try_from(len).expect("collection length exceeds u64::MAX");
    let zone = u64::MAX - (u64::MAX % bound);
    loop {
        let value = OsRng.next_u64();
        if value < zone {
            return (value % bound) as usize;
        }
    }
}

fn set_path(current: &mut Json, segments: &[&str], value: Json) {
    let Some((segment, rest)) = segments.split_first() else {
        *current = value;
        return;
    };

    if let Ok(index) = segment.parse::<usize>() {
        if !matches!(current, Json::Array(_)) {
            *current = Json::Array(Vec::new());
        }
        let Json::Array(values) = current else {
            unreachable!();
        };
        if values.len() <= index {
            values.resize(index + 1, Json::Null);
        }
        set_path(&mut values[index], rest, value);
    } else {
        if !matches!(current, Json::Object(_)) {
            *current = Json::Object(BTreeMap::new());
        }
        let Json::Object(values) = current else {
            unreachable!();
        };
        let child = values.entry((*segment).to_owned()).or_insert(Json::Null);
        set_path(child, rest, value);
    }
}

fn get_mut<'a>(value: &'a mut Json, path: &str) -> Option<&'a mut Json> {
    if path.is_empty() {
        return Some(value);
    }
    let mut current = value;
    for segment in path.split('.') {
        current = match current {
            Json::Object(values) => values.get_mut(segment)?,
            Json::Array(values) => values.get_mut(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

fn forget_path(current: &mut Json, segments: &[&str]) -> bool {
    let Some((segment, rest)) = segments.split_first() else {
        return false;
    };
    if rest.is_empty() {
        return match current {
            Json::Object(values) => values.remove(*segment).is_some(),
            Json::Array(values) => segment
                .parse::<usize>()
                .ok()
                .filter(|index| *index < values.len())
                .is_some_and(|index| {
                    values[index] = Json::Null;
                    true
                }),
            _ => false,
        };
    }
    match current {
        Json::Object(values) => values
            .get_mut(*segment)
            .is_some_and(|child| forget_path(child, rest)),
        Json::Array(values) => segment
            .parse::<usize>()
            .ok()
            .and_then(|index| values.get_mut(index))
            .is_some_and(|child| forget_path(child, rest)),
        _ => false,
    }
}

fn dot_into(value: &Json, prefix: &str, output: &mut BTreeMap<String, Json>) {
    match value {
        Json::Object(values) if !values.is_empty() => {
            for (key, child) in values {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                dot_into(child, &path, output);
            }
        }
        Json::Array(values) if !values.is_empty() => {
            for (index, child) in values.iter().enumerate() {
                let path = if prefix.is_empty() {
                    index.to_string()
                } else {
                    format!("{prefix}.{index}")
                };
                dot_into(child, &path, output);
            }
        }
        _ if !prefix.is_empty() => {
            output.insert(prefix.to_owned(), value.clone());
        }
        _ => {}
    }
}

fn flatten_into(value: &Json, output: &mut Vec<Json>) {
    match value {
        Json::Object(values) => {
            for child in values.values() {
                flatten_into(child, output);
            }
        }
        Json::Array(values) => {
            for child in values {
                flatten_into(child, output);
            }
        }
        _ => output.push(value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::Arr;
    use crate::Json;
    use std::collections::BTreeMap;

    fn object(entries: impl IntoIterator<Item = (&'static str, Json)>) -> Json {
        Json::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
        )
    }

    #[test]
    fn handles_typed_collections() {
        assert_eq!(Arr::collapse([[1, 2], [3, 4]]), vec![1, 2, 3, 4]);
        assert_eq!(Arr::take(&[1, 2, 3, 4], -2), vec![3, 4]);
        assert_eq!(Arr::join(&["A", "B", "C"], ", ", " and "), "A, B and C");
        assert_eq!(Arr::prepend(vec![2, 3], 1), vec![1, 2, 3]);
        assert_eq!(Arr::sort_desc(vec![2, 1, 3]), vec![3, 2, 1]);
    }

    #[test]
    fn filters_and_maps() {
        let values = [10, 20, 30, 40];
        assert_eq!(
            Arr::first_where(&values, |value, _| *value >= 20),
            Some(&20)
        );
        assert_eq!(Arr::last_where(&values, |value, _| *value >= 20), Some(&40));
        assert_eq!(
            Arr::map(&values, |value, index| value + index as i32),
            vec![10, 21, 32, 43]
        );
        assert_eq!(
            Arr::where_(&values, |value, _| *value >= 30),
            vec![&30, &40]
        );
        assert!(Arr::every(&values, |value, _| *value >= 10));
        assert!(Arr::some(&values, |value, _| *value == 30));
    }

    #[test]
    fn handles_string_keyed_maps() {
        let map = BTreeMap::from([("name".to_owned(), "Desk"), ("price".to_owned(), "100")]);
        assert!(Arr::exists(&map, "name"));
        assert_eq!(
            Arr::only(&map, ["name"]),
            BTreeMap::from([("name".to_owned(), "Desk")])
        );
        assert_eq!(
            Arr::except(&map, ["price"]),
            BTreeMap::from([("name".to_owned(), "Desk")])
        );
    }

    #[test]
    fn supports_dot_notation() {
        let mut value = object([(
            "products",
            object([("desk", object([("price", Json::from(100_u64))]))]),
        )]);

        assert_eq!(Arr::integer(&value, "products.desk.price"), Some(100));
        assert!(Arr::has(&value, "products.desk.price"));

        Arr::set(&mut value, "products.desk.stock", Json::from(5_u64));
        assert_eq!(Arr::integer(&value, "products.desk.stock"), Some(5));

        let pulled = Arr::pull(&mut value, "products.desk.stock");
        assert_eq!(pulled, Some(Json::from(5_u64)));
        assert!(!Arr::has(&value, "products.desk.stock"));
    }

    #[test]
    fn dots_and_undots_nested_json() {
        let value = object([(
            "products",
            object([("desk", object([("price", Json::from(100_u64))]))]),
        )]);
        let dotted = Arr::dot(&value);
        assert_eq!(
            dotted.get("products.desk.price"),
            Some(&Json::from(100_u64))
        );
        assert_eq!(Arr::undot(dotted), value);
    }

    #[test]
    fn plucks_and_flattens() {
        let values = vec![
            object([("name", Json::from("Desk"))]),
            object([("name", Json::from("Chair"))]),
        ];
        assert_eq!(
            Arr::pluck(&values, "name"),
            vec![Json::from("Desk"), Json::from("Chair")]
        );

        let nested = Json::Array(vec![
            Json::from(1_u64),
            Json::Array(vec![Json::from(2_u64)]),
        ]);
        assert_eq!(
            Arr::flatten(&nested),
            vec![Json::from(1_u64), Json::from(2_u64)]
        );
    }

    #[test]
    fn cross_joins_values() {
        assert_eq!(
            Arr::cross_join(&[vec![1, 2], vec![10, 20]]),
            vec![vec![1, 10], vec![1, 20], vec![2, 10], vec![2, 20]]
        );
    }
}
