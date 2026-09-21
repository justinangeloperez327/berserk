/// Generate hydration and presentation attributes from one field mapping.
///
/// Use inside `impl Model`. Fields use their Rust names as column names unless
/// explicitly renamed. Field types are inferred from the struct; only field
/// values are cloned for presentation, never the model itself.
///
/// ```
/// use claw_orm::{model_fields, Model, Value};
/// struct User { id: i64, name: String, password_hash: String }
/// impl Model for User {
///     const TABLE: &'static str = "users";
///     const HIDDEN: &'static [&'static str] = &["password_hash"];
///     model_fields! { id, name => "display_name", password_hash }
///     fn key(&self) -> Value { self.id.into() }
/// }
/// ```
///
/// `HIDDEN` names refer to mapped column names. This mapping does not change
/// `FILLABLE` or opt a model into `PersistableModel::save`.
#[macro_export]
macro_rules! model_fields {
    ($($field:ident $(=> $column:literal)?),+ $(,)?) => {
        fn from_row(row: &$crate::Row) -> $crate::Result<Self> {
            Ok(Self {
                $($field: $crate::field(row, $crate::model_fields!(@column $field $(=> $column)?))?),+
            })
        }

        fn attributes(&self) -> $crate::Attributes {
            ::std::collections::BTreeMap::from([
                $((
                    $crate::model_fields!(@column $field $(=> $column)?).to_owned(),
                    $crate::Value::from(self.$field.clone()),
                )),+
            ])
        }
    };
    (@column $field:ident => $column:literal) => { $column };
    (@column $field:ident) => { stringify!($field) };
}
