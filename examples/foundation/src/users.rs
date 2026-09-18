use berserk::{
    claw::{field, IntoInsert, IntoUpdate, Model, Row, Transaction, Value},
    response, ApiResource, Error, FormRequest, FromJson, IntoResponse, Json, Request,
    ResourceCollection, Response, Result, ValidationErrors, ValidationResult,
};

pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
}
impl Model for User {
    const TABLE: &'static str = "users";
    const FILLABLE: &'static [&'static str] = &["name", "email"];
    fn from_row(row: &Row) -> berserk::database::Result<Self> {
        Ok(Self {
            id: field(row, "id")?,
            name: field(row, "name")?,
            email: field(row, "email")?,
        })
    }
    fn key(&self) -> Value {
        self.id.into()
    }
    fn parse_route_key(value: &str) -> Option<Value> {
        value.parse::<i64>().ok().map(Into::into)
    }
}
impl ApiResource for User {
    fn to_resource(&self) -> Json {
        Json::Object(
            [
                ("id".into(), self.id.into()),
                ("name".into(), self.name.clone().into()),
                ("email".into(), self.email.clone().into()),
            ]
            .into(),
        )
    }
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

pub struct Users;
impl Users {
    pub fn index() -> Result<Response> {
        ResourceCollection::page(
            User::query()
                .order_by("id", berserk::claw::Direction::Asc)
                .paginate(20)?,
        )
        .into_response()
    }
    pub fn store(request: Request) -> Result<Response> {
        let input = request.validate::<UserInput>()?;
        let user = Transaction::run(|| User::create(input))?;
        response().status(201).json(user)
    }
    pub fn show(id: u64) -> Result<Response> {
        response().json(Self::find(id)?)
    }
    pub fn update(id: u64, request: Request) -> Result<Response> {
        let mut user = Self::find(id)?;
        let input = request.validate::<UserInput>()?;
        user.update(input)?;
        response().json(user)
    }
    pub fn destroy(id: u64) -> Result<Response> {
        Self::find(id)?.delete()?;
        response().no_content()
    }

    fn find(id: u64) -> Result<User> {
        let key = i64::try_from(id).map_err(|_| Error::bad_request("Invalid route parameter"))?;
        User::find(key)?.ok_or_else(Error::not_found)
    }
}
