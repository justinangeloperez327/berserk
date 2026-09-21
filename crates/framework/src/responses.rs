use crate::{
    presentation::ResponseData, IntoResponse, Resource, ResourceCollection, Response, Result,
};

/// Builds a response, preserving header errors until the response is completed.
#[derive(Debug)]
#[must_use]
pub struct ResponseFactory {
    status: Option<u16>,
    head: Result<Response>,
}

pub fn response() -> ResponseFactory {
    ResponseFactory {
        status: None,
        head: Ok(Response::empty()),
    }
}

/// Redirects to an absolute path on the same origin with status 302.
pub fn redirect(location: &str) -> Result<Response> {
    response().redirect(location)
}

#[cfg(feature = "view")]
pub fn view<D, Kind>(view: &str, data: D) -> Result<Response>
where
    D: crate::views::ViewData<Kind>,
{
    Response::view::<D, Kind>(view, data)
}

impl ResponseFactory {
    pub fn status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.head = self.head.and_then(|head| head.header(name, value));
        self
    }

    /// Serializes a Claw model's visible attributes or an explicit API resource,
    /// without adding a data envelope.
    pub fn json<Kind>(self, value: impl ResponseData<Kind>) -> Result<Response> {
        self.finish(Response::json(&value.response_data()?)?)
    }

    pub fn text(self, value: impl Into<String>) -> Result<Response> {
        self.finish(Response::text(value))
    }

    #[cfg(feature = "view")]
    pub fn view<D, Kind>(self, view: &str, data: D) -> Result<Response>
    where
        D: crate::views::ViewData<Kind>,
    {
        self.finish(Response::view::<D, Kind>(view, data)?)
    }

    pub fn empty(self) -> Result<Response> {
        self.finish(Response::empty())
    }

    pub fn created<Kind>(self, value: impl ResponseData<Kind>) -> Result<Response> {
        self.finish(Response::created(&value.response_data()?)?)
    }

    pub fn no_content(self) -> Result<Response> {
        self.finish(Response::no_content())
    }

    pub fn resource<Kind>(self, value: impl ResponseData<Kind>) -> Result<Response> {
        self.finish(Resource::new(value.response_data()?).into_response()?)
    }

    pub fn collection<T: ResponseData<Kind>, Kind>(
        self,
        values: impl IntoIterator<Item = T>,
    ) -> Result<Response> {
        self.finish(ResourceCollection::new::<Kind>(values).into_response()?)
    }

    /// Only same-origin absolute paths are accepted. Escape dynamic path segments first.
    pub fn redirect(self, location: &str) -> Result<Response> {
        if !location.starts_with('/')
            || location.starts_with("//")
            || location.chars().any(|character| {
                character == '\\' || character.is_control() || character.is_whitespace()
            })
        {
            return Err(crate::ConfigError::new(
                "redirect",
                "expected a same-origin absolute path without whitespace or backslashes",
            )
            .into());
        }
        let status = self.status.unwrap_or(302);
        if !matches!(status, 301 | 302 | 303 | 307 | 308) {
            return Err(crate::ConfigError::new("redirect", "invalid redirect status").into());
        }
        self.finish(Response::empty().status(status))?
            .header("location", location)
    }

    fn finish(self, mut response: Response) -> Result<Response> {
        let head = self.head?;
        for (name, value) in head.headers().iter() {
            response = response.header(name, value)?;
        }
        if let Some(status) = self.status {
            response = response.status(status);
        }
        response.into_response()
    }
}
