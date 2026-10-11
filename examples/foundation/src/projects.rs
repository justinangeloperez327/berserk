use crate::{
    domain::{Project, Task},
    integrations,
};
use berserk::{claw::Direction, presentation::ResponseData, response, Json, Response, Result};

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
        let tasks = Project::tasks()
            .query_for(&project)?
            .order_by("id", Direction::Asc)
            .get()?;
        let body = Json::Object(
            [
                ("project".into(), project.response_data()?),
                ("tasks".into(), tasks.response_data()?),
            ]
            .into(),
        );
        response().json(&body)
    }

    pub fn open_tasks(project: Project) -> Result<Response> {
        let tasks = Task::open()
            .where_("project_id", project.id)
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
        integrations::dispatch_task_integrated(
            &request,
            integrations::TaskIntegrated {
                task_id: task.id,
                project_id: task.project_id,
            },
        )?;
        integrations::queue_snapshot(&request, task.project_id)?;
        integrations::notify_snapshot(&request, "operator@example.test", task.title.clone())?;
        response().json(task)
    }

    pub fn show(task: Task) -> Result<Response> {
        let comments = Task::comments()
            .query_for(&task)?
            .order_by("id", Direction::Asc)
            .get()?;
        let body = Json::Object(
            [
                ("task".into(), task.response_data()?),
                ("comments".into(), comments.response_data()?),
            ]
            .into(),
        );
        response().json(&body)
    }
}
