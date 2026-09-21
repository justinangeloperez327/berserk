//! Internal adapters selected by type inference, like the controller adapters.
//! Applications implement `Model` or an intentional `ApiResource`, not these traits.
use crate::{ApiResource, Json, Result};

pub struct Explicit;
#[cfg(feature = "claw")]
pub struct Record;
#[cfg(feature = "claw")]
pub struct BorrowedRecord;
#[cfg(feature = "claw")]
pub struct Records;
#[cfg(feature = "claw")]
pub struct OptionalRecord;

pub trait ResponseData<Kind> {
    fn response_data(&self) -> Result<Json>;
}

impl<T: ApiResource + ?Sized> ResponseData<Explicit> for T {
    fn response_data(&self) -> Result<Json> {
        Ok(self.to_resource())
    }
}

#[cfg(feature = "claw")]
fn model_json<M: claw_orm::Model>(model: &M) -> Result<Json> {
    Ok(Json::Object(
        model
            .visible_attributes()
            .into_iter()
            .map(|(name, value)| scalar_json(value).map(|value| (name, value)))
            .collect::<Result<_>>()?,
    ))
}

#[cfg(feature = "claw")]
fn scalar_json(value: claw_orm::Value) -> Result<Json> {
    use claw_orm::Value;
    Ok(match value {
        Value::Null => Json::Null,
        Value::Bool(value) => value.into(),
        Value::I64(value) => value.into(),
        Value::U64(value) => value.into(),
        Value::F64(value) => {
            // JSON has no NaN or infinity. Fail instead of emitting invalid JSON
            // or silently changing a model value to null.
            Json::parse(value.to_string().as_bytes()).map_err(|_| {
                crate::ConfigError::new(
                    "model presentation",
                    "non-finite number cannot be encoded as JSON",
                )
            })?
        }
        Value::Text(value) => value.into(),
        Value::Bytes(value) => Json::Array(
            value
                .into_iter()
                .map(|byte| Json::from(u64::from(byte)))
                .collect(),
        ),
    })
}

#[cfg(feature = "claw")]
impl<M: claw_orm::Model> ResponseData<Record> for M {
    fn response_data(&self) -> Result<Json> {
        model_json(self)
    }
}

#[cfg(feature = "claw")]
impl<M: claw_orm::Model> ResponseData<BorrowedRecord> for &M {
    fn response_data(&self) -> Result<Json> {
        model_json(*self)
    }
}

#[cfg(feature = "claw")]
impl<M: claw_orm::Model> ResponseData<Records> for [M] {
    fn response_data(&self) -> Result<Json> {
        Ok(Json::Array(
            self.iter().map(model_json).collect::<Result<_>>()?,
        ))
    }
}

#[cfg(feature = "claw")]
impl<M: claw_orm::Model> ResponseData<Records> for Vec<M> {
    fn response_data(&self) -> Result<Json> {
        <[M] as ResponseData<Records>>::response_data(self.as_slice())
    }
}

#[cfg(feature = "claw")]
impl<M: claw_orm::Model> ResponseData<Records> for claw_orm::Collection<M> {
    fn response_data(&self) -> Result<Json> {
        <[M] as ResponseData<Records>>::response_data(self.as_ref())
    }
}

#[cfg(feature = "claw")]
impl<M: claw_orm::Model> ResponseData<Records> for &claw_orm::Collection<M> {
    fn response_data(&self) -> Result<Json> {
        <claw_orm::Collection<M> as ResponseData<Records>>::response_data(self)
    }
}

#[cfg(feature = "claw")]
impl<M: claw_orm::Model> ResponseData<OptionalRecord> for Option<M> {
    fn response_data(&self) -> Result<Json> {
        self.as_ref().map_or(Ok(Json::Null), model_json)
    }
}
