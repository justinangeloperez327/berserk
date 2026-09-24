use claw_orm::Collection;

#[test]
fn collection_remains_a_thin_rust_collection() {
    let mut values = Collection::from_vec(vec![1, 2, 3]);
    assert_eq!(values.len(), 3);
    assert!(!values.is_empty());
    assert_eq!(values.first(), Some(&1));
    assert_eq!(values.last(), Some(&3));
    assert_eq!(values.get(1), Some(&2));

    for value in &mut values { *value *= 2; }
    assert_eq!(values.as_ref(), &[2, 4, 6]);

    values.push(8);
    let filtered = values.filter(|value| *value >= 4);
    let mapped = filtered.map(|value| value.to_string());
    assert_eq!(mapped.into_vec(), vec!["4", "6", "8"]);
}

#[test]
fn collection_supports_owned_and_borrowed_iteration_without_model_bounds() {
    let values: Collection<_> = ["a", "b", "c"].into_iter().collect();
    let borrowed: Vec<_> = (&values).into_iter().copied().collect();
    assert_eq!(borrowed, vec!["a", "b", "c"]);

    let owned: Vec<_> = values.into_iter().collect();
    assert_eq!(owned, vec!["a", "b", "c"]);
}
