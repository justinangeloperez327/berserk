use crate::{ApiResource, IntoResponse, Json, Resource, ResourceCollection, Response, Result};

pub struct ResponseFactory;
pub fn response() -> ResponseFactory {
    ResponseFactory
}
impl ResponseFactory {
    pub fn json(self, value: &Json) -> Result<Response> {
        Response::json(value)
    }
    pub fn text(self, value: impl Into<String>) -> Response {
        Response::text(value)
    }
    pub fn created(self, value: &Json) -> Result<Response> {
        Response::created(value)
    }
    pub fn no_content(self) -> Response {
        Response::no_content()
    }
    pub fn resource<T: ApiResource>(self, value: T) -> Result<Response> {
        Resource::new(value).into_response()
    }
    pub fn collection<T: ApiResource>(self, values: Vec<T>) -> Result<Response> {
        ResourceCollection::new(values).into_response()
    }
}
