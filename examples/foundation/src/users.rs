use berserk::{
    auth::{Ability, Decision, Policy, Principal},
    claw::{Direction, IntoInsert, IntoUpdate, Transaction, Value},
    response, view_data, CrudController, Error, FormRequest, FromJson, Json, Model, Request,
    Response, Result, ValidationErrors, ValidationResult,
};
use std::path::PathBuf;

#[derive(Model)]
#[table("users")]
#[has_many(Post, "posts", foreign_key = "user_id")]
#[has_one(Profile, "profile", foreign_key = "user_id")]
#[belongs_to_many(
    Role,
    "roles",
    pivot = "role_user",
    foreign_pivot_key = "user_id",
    related_pivot_key = "role_id"
)]
pub struct User {
    #[primary_key]
    pub id: i64,

    #[fillable]
    pub name: String,

    #[fillable]
    pub email: String,
}

#[derive(Model)]
#[table("posts")]
#[belongs_to(User, "user", foreign_key = "user_id")]
pub struct Post {
    #[primary_key]
    pub id: i64,

    #[fillable]
    pub user_id: i64,

    #[fillable]
    pub title: String,
}

#[derive(Model)]
#[table("profiles")]
pub struct Profile {
    #[primary_key]
    pub id: i64,

    #[fillable]
    pub user_id: i64,
}

#[derive(Model)]
#[table("roles")]
pub struct Role {
    #[primary_key]
    pub id: i64,

    #[fillable]
    pub name: String,
}

pub struct UserInput {
    name: String,
    email: String,
}

impl FromJson for UserInput {
    fn from_json(value: &Json) -> std::result::Result<Self, ValidationErrors> {
        let name = value.get("name").and_then(Json::as_str);
        let email = value.get("email").and_then(Json::as_str);
        let mut errors = ValidationErrors::default();
        if name.is_none() {
            errors.add("name", "string", "A name string is required.");
        }
        if email.is_none() {
            errors.add("email", "string", "An email string is required.");
        }
        errors.finish()?;
        Ok(Self {
            name: name.unwrap_or_default().into(),
            email: email.unwrap_or_default().into(),
        })
    }
}

impl FormRequest for UserInput {
    fn sanitize(&mut self) {
        self.name = self.name.trim().into();
        self.email = self.email.trim().to_ascii_lowercase();
    }

    fn authorize_request(&self, request: &Request) -> Result<()> {
        if request.can("users.manage") {
            Ok(())
        } else {
            Err(Error::forbidden())
        }
    }

    fn validate(&self) -> ValidationResult {
        let mut errors = ValidationErrors::default();
        errors.length("name", &self.name, 1, 100);
        errors.email("email", &self.email);
        errors.finish()
    }

    fn validate_request(&self, request: &Request) -> Result<()> {
        let mut query = User::where_("email", self.email.clone());
        if let Some(Ok(id)) = request.param_as::<i64>("id") {
            query = query.where_op("id", "!=", id);
        }
        if query.exists()? {
            let mut errors = ValidationErrors::default();
            errors.add("email", "unique", "This email is already in use.");
            return Err(Error::Input(berserk::input::InputError::Fields(errors)));
        }
        Ok(())
    }
}

impl IntoInsert<User> for UserInput {
    fn into_insert(self) -> berserk::database::Result<Vec<(String, Value)>> {
        Ok(vec![
            ("name".into(), self.name.into()),
            ("email".into(), self.email.into()),
        ])
    }
}

impl IntoUpdate<User> for UserInput {
    fn into_update(self) -> berserk::database::Result<Vec<(String, Value)>> {
        self.into_insert()
    }
}

pub struct UserPolicy;

impl Policy<User> for UserPolicy {
    fn authorize(&self, principal: &Principal, action: &Ability, user: &User) -> Decision {
        if action.as_str() != "users.view" {
            return Decision::Deny;
        }
        if principal.has_role("admin") || principal.subject() == format!("user:{}", user.id) {
            Decision::Allow
        } else {
            Decision::Deny
        }
    }
}

pub struct Users;

impl Users {
    pub fn browse() -> Result<Response> {
        let users = User::query()
            .with(["posts", "roles"])
            .order_by("id", Direction::Asc)
            .get()?;
        let context = view_data!["users" => users];
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("app/views");
        Ok(Response::html(berserk::axe::render_from(
            root,
            "users/index",
            &context,
        )?))
    }

    pub fn policy_show(user: User, request: Request) -> Result<Response> {
        request.authorize(&UserPolicy, "users.view", &user)?;
        response().json(user)
    }
}

impl CrudController for Users {
    type Model = User;
    type Create = UserInput;
    type Update = UserInput;

    fn index(&self) -> Result<Response> {
        let users = User::query()
            .with(["posts", "profile", "roles"])
            .order_by("id", Direction::Asc)
            .paginate(20)?;
        response().json(&users)
    }

    fn store(&self, input: Self::Create) -> Result<Response> {
        let user = Transaction::run(|| User::create(input))?;
        response().status(201).json(user)
    }

    fn show(&self, user: Self::Model) -> Result<Response> {
        response().json(user)
    }

    fn update(&self, mut user: Self::Model, input: Self::Update) -> Result<Response> {
        user.update(input)?;
        response().json(user)
    }

    fn destroy(&self, user: Self::Model) -> Result<Response> {
        user.delete()?;
        response().no_content()
    }
}
