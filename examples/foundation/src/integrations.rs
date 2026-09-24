use berserk::{
    cache::Cache,
    events::{Event, EventBus},
    jobs::{Job, JobContext, JobQueue, RetryPolicy},
    notifications::{EmailAddress, MailMessage, Notification, Notifier, Recipient},
    storage::{Storage, StoragePath},
    Request, Result,
};
use std::{io::Cursor, sync::Arc};

pub struct TaskCreated {
    pub task_id: i64,
    pub project_id: i64,
}
impl Event for TaskCreated { const NAME: &'static str = "task.created"; }

pub struct ProjectSnapshotJob {
    pub project_id: i64,
    pub storage: Arc<dyn Storage>,
}
impl Job for ProjectSnapshotJob {
    fn name(&self) -> &'static str { "project.snapshot" }
    fn handle(&mut self, _context: &JobContext) -> berserk::jobs::Result<()> {
        let path = StoragePath::new(format!("projects/{}/snapshot.txt", self.project_id))
            .map_err(|e| berserk::jobs::JobError::new(berserk::jobs::ErrorKind::Handler, e.to_string()))?;
        let mut body = Cursor::new(format!("project={}", self.project_id).into_bytes());
        self.storage.put(&path, &mut body)
            .map_err(|e| berserk::jobs::JobError::new(berserk::jobs::ErrorKind::Handler, e.to_string()))?;
        Ok(())
    }
}

pub struct TaskAssignedMail { pub title: String }
impl Notification for TaskAssignedMail {
    fn mail(&self, recipient: &Recipient) -> berserk::notifications::Result<Option<MailMessage>> {
        let Some(to) = recipient.email_address() else { return Ok(None); };
        Ok(Some(MailMessage::text(
            EmailAddress::new("foundation@example.test")?,
            to.clone(),
            "Task assigned",
            format!("You were assigned: {}", self.title),
        )?))
    }
}

pub fn invalidate_project(request: &Request, project_id: i64) -> Result<()> {
    let cache = request.state::<Arc<dyn Cache>>()
        .ok_or_else(|| berserk::ConfigError::new("cache", "cache is not configured"))?;
    cache.forget(&format!("project:{project_id}"))
        .map_err(|e| berserk::ConfigError::new("cache", e.to_string()))?;
    Ok(())
}

pub fn dispatch_task_created(request: &Request, event: TaskCreated) -> Result<()> {
    let events = request.shared::<Arc<EventBus>>()?;
    events.dispatch(&event).map_err(|e| berserk::ConfigError::new("events", e.to_string()))?;
    Ok(())
}

pub fn queue_snapshot(request: &Request, project_id: i64) -> Result<()> {
    let queue = request.shared::<JobQueue>()?;
    let storage = request.shared::<Arc<dyn Storage>>()?;
    queue.dispatch(ProjectSnapshotJob { project_id, storage: (*storage).clone() }, RetryPolicy::none())
        .map_err(|e| berserk::ConfigError::new("jobs", e.to_string()))?;
    Ok(())
}

pub fn notify_assignment(request: &Request, email: &str, title: String) -> Result<()> {
    let notifier = request.shared::<Notifier>()?;
    let recipient = Recipient::new().email(
        EmailAddress::new(email).map_err(|e| berserk::ConfigError::new("notifications", e.to_string()))?
    );
    let report = notifier.send(&recipient, &TaskAssignedMail { title })
        .map_err(|e| berserk::ConfigError::new("notifications", e.to_string()))?;
    if !report.all_sent() {
        return Err(berserk::ConfigError::new("notifications", "task assignment delivery failed").into());
    }
    Ok(())
}
