use crate::{domain::{Project, Task}, integrations};
use berserk::{claw::Direction, response, Json, Response, Result};

pub struct Projects;

impl Projects {
    pub fn index() -> Result<Response> {
        let projects = Project::visible()
            .with(["tasks"])
            .order_by("id", Direction::Asc)
            .paginate(20)?;
        response().json(&projects)
    }

    pub fn show(project: Project) -> Result<Response> {
        let tasks = project.tasks()?.order_by("id", Direction::Asc).get()?;
        let body = Json::Object([
            ("project".into(), Json::from(project)),
            ("tasks".into(), Json::from(tasks)),
        ].into());
        response().json(&body)
    }

    pub fn open_tasks(project: Project) -> Result<Response> {
        let tasks = Task::where_("project_id", project.id)
            .where_("status", "open")
            .with(["comments"])
            .order_by("id", Direction::Asc)
            .paginate(20)?;
        response().json(&tasks)
    }
}

pub struct Tasks;
impl Tasks {
    pub fn integrate(task: Task, request: berserk::Request) -> Result<Response> {
        integrations::invalidate_project(&request, task.project_id)?;
        integrations::dispatch_task_created(&request, integrations::TaskCreated { task_id: task.id, project_id: task.project_id })?;
        integrations::queue_snapshot(&request, task.project_id)?;
        response().json(task)
    }

    pub fn show(task: Task) -> Result<Response> {
        let comments = task.comments()?.order_by("id", Direction::Asc).get()?;
        let body = Json::Object([
            ("task".into(), Json::from(task)),
            ("comments".into(), Json::from(comments)),
        ].into());
        response().json(&body)
    }
}
