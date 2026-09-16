use crate::{Request, Response, Result};
use std::str::FromStr;

/// Controller contract for REST resources exposed through API routes.
pub trait ApiResourceController: Send + Sync + 'static {
    type Id: FromStr + 'static;

    fn index(&self, request: Request) -> Result<Response>;
    fn store(&self, request: Request) -> Result<Response>;
    fn show(&self, id: Self::Id, request: Request) -> Result<Response>;
    fn update(&self, id: Self::Id, request: Request) -> Result<Response>;
    fn destroy(&self, id: Self::Id, request: Request) -> Result<Response>;
}

/// Controller contract for a full REST resource, including create/edit pages.
pub trait ResourceController: ApiResourceController {
    fn create(&self, request: Request) -> Result<Response>;
    fn edit(&self, id: Self::Id, request: Request) -> Result<Response>;
}
