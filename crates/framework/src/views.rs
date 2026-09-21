//! Framework-owned conversions. Axe only receives its own values and contexts.
use berserk_axe::{Context, Value};

pub struct Native;

pub trait ViewValue<Kind> {
    fn into_view_value(self) -> Value;
}

impl<T: Into<Value>> ViewValue<Native> for T {
    fn into_view_value(self) -> Value {
        self.into()
    }
}

pub fn value<T: ViewValue<Kind>, Kind>(value: T) -> Value {
    value.into_view_value()
}

pub trait ViewData<Kind> {
    fn into_view_data(self) -> Context;
}

impl<T: Into<Context>> ViewData<Native> for T {
    fn into_view_data(self) -> Context {
        self.into()
    }
}

#[cfg(feature = "claw")]
mod models {
    use super::*;
    use crate::presentation::{
        BorrowedNamedRecords, BorrowedRecord, NamedRecords, Record, Records,
    };
    use claw_orm::{
        Attributes, Collection, Loaded, Model, NamedRelation, NamedRelations, RelationCardinality,
    };

    pub struct BorrowedRecords;

    fn attributes_value(attributes: &Attributes) -> Value {
        Value::Object(
            attributes
                .iter()
                .map(|(name, value)| (name.clone(), scalar_value(value.clone())))
                .collect(),
        )
    }

    fn model_value<M: Model>(model: &M) -> Value {
        attributes_value(&model.visible_attributes())
    }

    fn relation_value(relation: &NamedRelation, parent_key: &claw_orm::Value) -> Value {
        let values = relation.get(parent_key).unwrap_or(&[]);
        match relation.cardinality() {
            RelationCardinality::Many => {
                Value::List(values.iter().map(attributes_value).collect())
            }
            RelationCardinality::One => values
                .first()
                .map(attributes_value)
                .unwrap_or(Value::Null),
        }
    }

    fn named_model_value<M: Model>(model: &M, relations: &NamedRelations) -> Value {
        let mut values = match model_value(model) {
            Value::Object(values) => values,
            _ => unreachable!("model view conversion always produces an object"),
        };
        let key = model.key();
        for relation in relations.iter() {
            values.insert(relation.name().to_owned(), relation_value(relation, &key));
        }
        Value::Object(values)
    }

    fn named_loaded_value<M: Model>(loaded: &Loaded<M, NamedRelations>) -> Value {
        Value::List(
            loaded
                .models
                .iter()
                .map(|model| named_model_value(model, &loaded.relations))
                .collect(),
        )
    }

    fn scalar_value(value: claw_orm::Value) -> Value {
        use claw_orm::Value as Scalar;
        match value {
            Scalar::Null => Value::Null,
            Scalar::Bool(value) => Value::Bool(value),
            Scalar::I64(value) => value.into(),
            Scalar::U64(value) => value.into(),
            Scalar::F64(value) => Value::Number(value.to_string()),
            Scalar::Text(value) => Value::Text(value),
            Scalar::Bytes(value) => Value::Bytes(value),
        }
    }

    impl<M: Model> ViewValue<Record> for M {
        fn into_view_value(self) -> Value {
            model_value(&self)
        }
    }

    impl<M: Model> ViewValue<BorrowedRecord> for &M {
        fn into_view_value(self) -> Value {
            model_value(self)
        }
    }

    impl<M: Model> ViewValue<Records> for Collection<M> {
        fn into_view_value(self) -> Value {
            Value::List(self.iter().map(model_value).collect())
        }
    }

    impl<M: Model> ViewValue<BorrowedRecords> for &Collection<M> {
        fn into_view_value(self) -> Value {
            Value::List(self.iter().map(model_value).collect())
        }
    }

    impl<M: Model> ViewValue<NamedRecords> for Loaded<M, NamedRelations> {
        fn into_view_value(self) -> Value {
            named_loaded_value(&self)
        }
    }

    impl<M: Model> ViewValue<BorrowedNamedRecords> for &Loaded<M, NamedRelations> {
        fn into_view_value(self) -> Value {
            named_loaded_value(self)
        }
    }

    macro_rules! model_data {
        ($kind:ty, $value:ty) => {
            impl<K: Into<String>, M: Model, const N: usize> ViewData<$kind> for [(K, $value); N] {
                fn into_view_data(self) -> Context {
                    self.into_iter()
                        .map(|(name, value)| {
                            (name, <$value as ViewValue<$kind>>::into_view_value(value))
                        })
                        .collect()
                }
            }
        };
    }

    model_data!(Record, M);
    model_data!(BorrowedRecord, &M);
    model_data!(Records, Collection<M>);
    model_data!(BorrowedRecords, &Collection<M>);
    model_data!(NamedRecords, Loaded<M, NamedRelations>);
    model_data!(BorrowedNamedRecords, &Loaded<M, NamedRelations>);
}

/// Build explicit heterogeneous view data using Berserk's model conversion.
#[macro_export]
macro_rules! view_data {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut context = $crate::axe::Context::new();
        $(let _ = context.insert($key, $crate::views::value($value));)*
        context
    }};
}

#[macro_export]
macro_rules! view_object {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut values = ::std::collections::BTreeMap::<String, $crate::axe::Value>::new();
        $(let _ = values.insert(($key).into(), $crate::views::value($value));)*
        $crate::axe::Value::Object(values)
    }};
}
