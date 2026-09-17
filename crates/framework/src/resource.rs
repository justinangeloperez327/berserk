use crate::{IntoResponse, Json, Response, Result};
use std::collections::BTreeMap;

/// Explicitly maps an internal value to its public API representation.
pub trait ApiResource {
    fn to_resource(&self) -> Json;
}

pub struct Resource<T>(pub T);

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
    links: BTreeMap<String, Json>,
    meta: BTreeMap<String, Json>,
}

impl<T> ResourceCollection<T> {
    pub fn new(values: Vec<T>) -> Self {
        Self {
            values,
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

impl<T: ApiResource> IntoResponse for ResourceCollection<T> {
    fn into_response(self) -> Result<Response> {
        let mut body = BTreeMap::from([(
            "data".into(),
            Json::Array(
                self.values
                    .iter()
                    .map(|value| value.to_resource())
                    .collect(),
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
    pub fn page(page: claw_orm::Page<T>) -> Self {
        let (number, per_page, total, last) =
            (page.page(), page.per_page(), page.total(), page.last_page());
        Self::new(page.into_items())
            .meta("current_page", number)
            .meta("per_page", per_page)
            .meta("total", total)
            .meta("last_page", last)
    }
}
