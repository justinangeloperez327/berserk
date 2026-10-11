use berserk::{
    events::Event,
    jobs::{Job, JobContext, RetryPolicy},
    notifications::{EmailAddress, MailMessage, Notification, Recipient},
    storage::{Storage, StoragePath},
    Request, Result,
};
use std::{io::Cursor, sync::Arc};

pub struct TaskIntegrated {
    pub task_id: i64,
    pub project_id: i64,
}
impl Event for TaskIntegrated {
    const NAME: &'static str = "task.integrated";
}

pub struct ProjectSnapshotJob {
    pub project_id: i64,
    pub storage: Arc<dyn Storage>,
}
impl Job for ProjectSnapshotJob {
    fn name(&self) -> &'static str {
        "project.snapshot"
    }
    fn handle(&mut self, _context: &JobContext) -> berserk::jobs::Result<()> {
        let path = StoragePath::new(format!("projects/{}/snapshot.txt", self.project_id)).map_err(
            |e| berserk::jobs::JobError::new(berserk::jobs::ErrorKind::Handler, e.to_string()),
        )?;
        let mut body = Cursor::new(format!("project={}", self.project_id).into_bytes());
        self.storage.put(&path, &mut body).map_err(|e| {
            berserk::jobs::JobError::new(berserk::jobs::ErrorKind::Handler, e.to_string())
        })?;
        Ok(())
    }
}

pub struct TaskSnapshotMail {
    pub title: String,
}
impl Notification for TaskSnapshotMail {
    fn mail(&self, recipient: &Recipient) -> berserk::notifications::Result<Option<MailMessage>> {
        let Some(to) = recipient.email_address() else {
            return Ok(None);
        };
        Ok(Some(MailMessage::text(
            EmailAddress::new("foundation@example.test")?,
            to.clone(),
            "Task snapshot requested",
            format!("Snapshot requested for: {}", self.title),
        )?))
    }
}

pub fn invalidate_project(request: &Request, project_id: i64) -> Result<()> {
    let cache = request.cache()?;
    cache
        .forget(&format!("project:{project_id}"))
        .map_err(|_| berserk::ConfigError::new("cache", "cache invalidation failed"))?;
    Ok(())
}

pub fn dispatch_task_integrated(request: &Request, event: TaskIntegrated) -> Result<()> {
    let events = request.events()?;
    events
        .dispatch(&event)
        .map_err(|_| berserk::ConfigError::new("events", "event dispatch failed"))?;
    Ok(())
}

pub fn queue_snapshot(request: &Request, project_id: i64) -> Result<()> {
    let queue = request.jobs()?;
    let storage = request.storage()?;
    queue
        .dispatch(
            ProjectSnapshotJob {
                project_id,
                storage,
            },
            RetryPolicy::none(),
        )
        .map_err(|_| berserk::ConfigError::new("jobs", "job dispatch failed"))?;
    Ok(())
}

pub fn notify_snapshot(request: &Request, email: &str, title: String) -> Result<()> {
    let notifier = request.notifications()?;
    let recipient = Recipient::new().email(
        EmailAddress::new(email)
            .map_err(|_| berserk::ConfigError::new("notifications", "invalid recipient email"))?,
    );
    let report = notifier
        .send(&recipient, &TaskSnapshotMail { title })
        .map_err(|_| berserk::ConfigError::new("notifications", "notification delivery failed"))?;
    if !report.all_sent() {
        return Err(berserk::ConfigError::new(
            "notifications",
            "snapshot notification delivery failed",
        )
        .into());
    }
    Ok(())
}
