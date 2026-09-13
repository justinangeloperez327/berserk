//! Bounded background jobs, retries, failed-job records, and deterministic scheduling.
#![forbid(unsafe_code)]

mod error;
mod failure;
mod queue;
mod schedule;

pub use error::{ErrorKind, JobError, Result};
pub use failure::{FailedJob, FailedJobStore, MemoryFailedJobs};
pub use queue::{JobQueue, QueueConfig, QueueSnapshot, RetryPolicy, ShutdownReport, WorkerPool};
pub use schedule::{ScheduleId, ScheduleReport, Scheduler};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobContext {
    attempt: u32,
}
impl JobContext {
    pub const fn attempt(&self) -> u32 {
        self.attempt
    }
}

pub trait Job: Send + 'static {
    fn name(&self) -> &'static str;
    fn handle(&mut self, context: &JobContext) -> Result<()>;
}
