use claw_orm::Collection;

#[test]
fn collection_behaves_like_rust_collection() {
    let values = Collection::from(vec![1, 2, 3, 4]);

    assert_eq!(values.len(), 4);
    assert_eq!(values.first(), Some(&1));
    assert_eq!(values.last(), Some(&4));
    assert_eq!(&values[..2], &[1, 2]);

    let mapped = values.clone().map(|value| value * 2);
    assert_eq!(mapped.as_ref(), &[2, 4, 6, 8]);

    let filtered = values.filter(|value| *value % 2 == 0);
    assert_eq!(filtered.as_ref(), &[2, 4]);
}
