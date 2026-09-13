use crate::{ErrorKind, JobError, Result};
use std::{collections::VecDeque, sync::Mutex};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailedJob {
    pub name: &'static str,
    pub attempts: u32,
    pub failed_at: u64,
    pub error: String,
}

pub trait FailedJobStore: Send + Sync {
    fn record(&self, failure: FailedJob) -> Result<()>;
}

pub struct MemoryFailedJobs {
    capacity: usize,
    failures: Mutex<VecDeque<FailedJob>>,
}
impl MemoryFailedJobs {
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(JobError::new(
                ErrorKind::Configuration,
                "failed-job capacity must be greater than zero",
            ));
        }
        Ok(Self {
            capacity,
            failures: Mutex::new(VecDeque::new()),
        })
    }
    pub fn all(&self) -> Result<Vec<FailedJob>> {
        Ok(self
            .failures
            .lock()
            .map_err(|_| JobError::new(ErrorKind::FailureStore, "failed-job store lock poisoned"))?
            .iter()
            .cloned()
            .collect())
    }
    pub fn clear(&self) -> Result<()> {
        self.failures
            .lock()
            .map_err(|_| JobError::new(ErrorKind::FailureStore, "failed-job store lock poisoned"))?
            .clear();
        Ok(())
    }
}
impl FailedJobStore for MemoryFailedJobs {
    fn record(&self, failure: FailedJob) -> Result<()> {
        let mut failures = self.failures.lock().map_err(|_| {
            JobError::new(ErrorKind::FailureStore, "failed-job store lock poisoned")
        })?;
        if failures.len() == self.capacity {
            failures.pop_front();
        }
        failures.push_back(failure);
        Ok(())
    }
}
