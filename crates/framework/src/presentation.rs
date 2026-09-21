//! Internal adapters selected by type inference, like the controller adapters.
//! Applications implement Model or an intentional ApiResource, not these traits.
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
#[cfg(feature = "claw")]
pub struct NamedRecords;
#[cfg(feature = "claw")]
pub struct BorrowedNamedRecords;

pub trait ResponseData<Kind> {
    fn response_data(&self) -> Result<Json>;
}

impl<T: ApiResource + ?Sized> ResponseData<Explicit> for T {
    fn response_data(&self) -> Result<Json> {
        Ok(self.to_resource())
    }
}

#[cfg(feature = "claw")]
fn attributes_json(attributes: &claw_orm::Attributes) -> Result<Json> {
    Ok(Json::Object(
        attributes
            .iter()
            .map(|(name, value)| scalar_json(value.clone()).map(|value| (name.clone(), value)))
            .collect::<Result<_>>()?,
    ))
}

#[cfg(feature = "claw")]
fn model_json<M: claw_orm::Model>(model: &M) -> Result<Json> {
    attributes_json(&model.visible_attributes())
}

#[cfg(feature = "claw")]
fn named_relation_json(
    relation: &claw_orm::NamedRelation,
    parent_key: &claw_orm::Value,
) -> Result<Json> {
    let values = relation.get(parent_key).unwrap_or(&[]);
    match relation.cardinality() {
        claw_orm::RelationCardinality::Many => Ok(Json::Array(
            values
                .iter()
                .map(attributes_json)
                .collect::<Result<Vec<_>>>()?,
        )),
        claw_orm::RelationCardinality::One => values
            .first()
            .map(attributes_json)
            .unwrap_or(Ok(Json::Null)),
    }
}

#[cfg(feature = "claw")]
fn named_model_json<M: claw_orm::Model>(
    model: &M,
    relations: &claw_orm::NamedRelations,
) -> Result<Json> {
    let mut object = match model_json(model)? {
        Json::Object(object) => object,
        _ => unreachable!("model presentation always produces an object"),
    };
    let key = model.key();
    for relation in relations.iter() {
        object.insert(
            relation.name().to_owned(),
            named_relation_json(relation, &key)?,
        );
    }
    Ok(Json::Object(object))
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

#[cfg(feature = "claw")]
impl<M: claw_orm::Model> ResponseData<NamedRecords>
    for claw_orm::Loaded<M, claw_orm::NamedRelations>
{
    fn response_data(&self) -> Result<Json> {
        Ok(Json::Array(
            self.models
                .iter()
                .map(|model| named_model_json(model, &self.relations))
                .collect::<Result<_>>()?,
        ))
    }
}

#[cfg(feature = "claw")]
impl<M: claw_orm::Model> ResponseData<BorrowedNamedRecords>
    for &claw_orm::Loaded<M, claw_orm::NamedRelations>
{
    fn response_data(&self) -> Result<Json> {
        <claw_orm::Loaded<M, claw_orm::NamedRelations> as ResponseData<NamedRecords>>::response_data(
            self,
        )
    }
}
