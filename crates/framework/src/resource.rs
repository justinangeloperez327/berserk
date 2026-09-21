use crate::{presentation::ResponseData, IntoResponse, Json, Response, Result};
use std::collections::BTreeMap;

/// Explicitly maps an internal value to its public API representation.
pub trait ApiResource {
    fn to_resource(&self) -> Json;
}

impl ApiResource for Json {
    fn to_resource(&self) -> Json {
        self.clone()
    }
}

impl<T: ApiResource> ApiResource for [T] {
    fn to_resource(&self) -> Json {
        Json::Array(self.iter().map(ApiResource::to_resource).collect())
    }
}

impl<T: ApiResource> ApiResource for Vec<T> {
    fn to_resource(&self) -> Json {
        self.as_slice().to_resource()
    }
}

#[cfg(feature = "claw")]
impl<T: ApiResource> ApiResource for claw_orm::Collection<T> {
    fn to_resource(&self) -> Json {
        self.as_ref().to_resource()
    }
}

impl<T: ApiResource> ApiResource for Option<T> {
    fn to_resource(&self) -> Json {
        self.as_ref().map_or(Json::Null, ApiResource::to_resource)
    }
}

pub struct Resource<T>(pub T);

// This explicit wrapper also disambiguates a legacy type implementing both
// Model and ApiResource: response().json(Resource::new(value)) chooses the
// intentional resource mapping. Its IntoResponse still adds the data envelope.
impl<T: ApiResource> ApiResource for Resource<T> {
    fn to_resource(&self) -> Json {
        self.0.to_resource()
    }
}

impl<T> Resource<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: ApiResource> IntoResponse for Resource<T> {
    fn into_response(self) -> Result<Response> {
        let body = Json::Object(BTreeMap::from([("data".into(), self.0.to_resource())]));
        Response::json(&body)?.into_response()
    }
}

pub struct ResourceCollection<T> {
    values: Vec<T>,
    serialize: fn(&T) -> Result<Json>,
    links: BTreeMap<String, Json>,
    meta: BTreeMap<String, Json>,
}

impl<T> ResourceCollection<T> {
    pub fn new<Kind>(values: impl IntoIterator<Item = T>) -> Self
    where
        T: ResponseData<Kind>,
    {
        Self {
            values: values.into_iter().collect(),
            serialize: <T as ResponseData<Kind>>::response_data,
            links: BTreeMap::new(),
            meta: BTreeMap::new(),
        }
    }
    pub fn link(mut self, name: impl Into<String>, value: impl Into<Json>) -> Self {
        self.links.insert(name.into(), value.into());
        self
    }
    pub fn meta(mut self, name: impl Into<String>, value: impl Into<Json>) -> Self {
        self.meta.insert(name.into(), value.into());
        self
    }
    pub fn into_inner(self) -> Vec<T> {
        self.values
    }
}

impl<T> IntoResponse for ResourceCollection<T> {
    fn into_response(self) -> Result<Response> {
        let mut body = BTreeMap::from([(
            "data".into(),
            Json::Array(
                self.values
                    .iter()
                    .map(self.serialize)
                    .collect::<Result<_>>()?,
            ),
        )]);
        if !self.links.is_empty() {
            body.insert("links".into(), Json::Object(self.links));
        }
        if !self.meta.is_empty() {
            body.insert("meta".into(), Json::Object(self.meta));
        }
        Response::json(&Json::Object(body))?.into_response()
    }
}

impl<T: ApiResource + ?Sized> ApiResource for &T {
    fn to_resource(&self) -> Json {
        (*self).to_resource()
    }
}
#[cfg(feature = "claw")]
impl<T> ResourceCollection<T> {
    pub fn page<Kind>(page: claw_orm::Page<T>) -> Self
    where
        T: ResponseData<Kind>,
    {
        let (number, per_page, total, last) =
            (page.page(), page.per_page(), page.total(), page.last_page());
        Self::new::<Kind>(page.into_collection())
            .meta("current_page", number)
            .meta("per_page", per_page)
            .meta("total", total)
            .meta("last_page", last)
    }
}
