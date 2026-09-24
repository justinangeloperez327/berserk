use berserk::Model;

#[derive(Model)]
#[table("projects")]
#[has_many(Task, "tasks", foreign_key = "project_id")]
pub struct Project {
    #[primary_key]
    pub id: i64,
    #[fillable]
    pub owner_id: i64,
    #[fillable]
    pub name: String,
    #[fillable]
    pub status: String,
}

#[derive(Model)]
#[table("tasks")]
#[belongs_to(Project, "project", foreign_key = "project_id")]
#[has_many(Comment, "comments", foreign_key = "task_id")]
pub struct Task {
    #[primary_key]
    pub id: i64,
    #[fillable]
    pub project_id: i64,
    #[fillable]
    pub assignee_id: Option<i64>,
    #[fillable]
    pub title: String,
    #[fillable]
    pub status: String,
}

#[derive(Model)]
#[table("comments")]
#[belongs_to(Task, "task", foreign_key = "task_id")]
pub struct Comment {
    #[primary_key]
    pub id: i64,
    #[fillable]
    pub task_id: i64,
    #[fillable]
    pub user_id: i64,
    #[fillable]
    pub body: String,
}

impl Project {
    pub fn visible() -> berserk::claw::ModelQuery<Self> {
        Self::query().where_in("status", ["active", "completed"])
    }
}

impl Task {
    pub fn open() -> berserk::claw::ModelQuery<Self> {
        Self::where_("status", "open")
    }
}
