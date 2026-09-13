use crate::{ErrorKind, Job, JobError, JobQueue, Result, RetryPolicy};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ScheduleId(u64);
type Factory = Arc<dyn Fn() -> Box<dyn Job> + Send + Sync>;
struct Entry {
    id: ScheduleId,
    interval: Duration,
    next_due: u64,
    retry: RetryPolicy,
    factory: Factory,
}
#[derive(Default)]
struct State {
    next_id: u64,
    entries: Vec<Entry>,
}
#[derive(Default)]
pub struct Scheduler {
    state: Mutex<State>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduleReport {
    pub due: usize,
    pub dispatched: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn every<J, F>(
        &self,
        interval: Duration,
        retry: RetryPolicy,
        factory: F,
    ) -> Result<ScheduleId>
    where
        J: Job,
        F: Fn() -> J + Send + Sync + 'static,
    {
        if interval.is_zero() {
            return Err(JobError::new(
                ErrorKind::Configuration,
                "schedule interval must be greater than zero",
            ));
        }
        let retry = retry.validate()?;
        let current = now_millis()?;
        let millis: u64 = interval.as_millis().try_into().map_err(|_| {
            JobError::new(ErrorKind::Configuration, "schedule interval is too large")
        })?;
        let next_due = current
            .checked_add(millis)
            .ok_or_else(|| JobError::new(ErrorKind::Capacity, "schedule time overflow"))?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| JobError::new(ErrorKind::Closed, "scheduler lock poisoned"))?;
        state.next_id = state.next_id.checked_add(1).ok_or_else(|| {
            JobError::new(
                ErrorKind::Capacity,
                "schedule identifier capacity exhausted",
            )
        })?;
        let id = ScheduleId(state.next_id);
        let factory: Factory = Arc::new(move || Box::new(factory()));
        state.entries.push(Entry {
            id,
            interval,
            next_due,
            retry,
            factory,
        });
        Ok(id)
    }
    pub fn forget(&self, id: ScheduleId) -> Result<bool> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JobError::new(ErrorKind::Closed, "scheduler lock poisoned"))?;
        let before = state.entries.len();
        state.entries.retain(|entry| entry.id != id);
        Ok(before != state.entries.len())
    }
    pub fn run_due(&self, queue: &JobQueue) -> Result<ScheduleReport> {
        self.run_due_at(queue, now_millis()?)
    }
    pub fn run_due_at(&self, queue: &JobQueue, current_millis: u64) -> Result<ScheduleReport> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| JobError::new(ErrorKind::Closed, "scheduler lock poisoned"))?;
        let mut report = ScheduleReport {
            due: 0,
            dispatched: 0,
        };
        for entry in &mut state.entries {
            if entry.next_due > current_millis {
                continue;
            }
            report.due += 1;
            let millis: u64 = entry.interval.as_millis().try_into().map_err(|_| {
                JobError::new(ErrorKind::Capacity, "schedule interval is too large")
            })?;
            let next_due = current_millis
                .checked_add(millis)
                .ok_or_else(|| JobError::new(ErrorKind::Capacity, "schedule time overflow"))?;
            let job = catch_unwind(AssertUnwindSafe(|| (entry.factory)())).map_err(|_| {
                JobError::new(ErrorKind::HandlerPanic, "scheduled job factory panicked")
            })?;
            queue.dispatch_boxed(job, entry.retry)?;
            report.dispatched += 1;
            entry.next_due = next_due;
        }
        Ok(report)
    }
}
fn now_millis() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| JobError::new(ErrorKind::Clock, "system clock is before Unix epoch"))?
        .as_millis()
        .try_into()
        .map_err(|_| JobError::new(ErrorKind::Clock, "system clock value overflow"))
}
