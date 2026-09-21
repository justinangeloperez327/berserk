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
    use crate::presentation::{BorrowedRecord, Record, Records};
    use claw_orm::{Collection, Model};

    pub struct BorrowedRecords;

    fn model_value<M: Model>(model: &M) -> Value {
        Value::Object(
            model
                .visible_attributes()
                .into_iter()
                .map(|(name, value)| (name, scalar_value(value)))
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

    // Restrict these array adapters to model values. Ordinary Axe-compatible
    // arrays continue through Into<Context>, with no competing native adapter.
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
