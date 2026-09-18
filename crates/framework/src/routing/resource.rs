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

/// Model-bound API actions with typed validated input.
#[cfg(feature = "claw")]
pub trait CrudController: Send + Sync + 'static {
    type Model: claw_orm::Model + 'static;
    type Create: crate::FormRequest + 'static;
    type Update: crate::FormRequest + 'static;
    fn index(&self) -> crate::Result<crate::Response>;
    fn store(&self, input: Self::Create) -> crate::Result<crate::Response>;
    fn show(&self, model: Self::Model) -> crate::Result<crate::Response>;
    fn update(&self, model: Self::Model, input: Self::Update) -> crate::Result<crate::Response>;
    fn destroy(&self, model: Self::Model) -> crate::Result<crate::Response>;
}
