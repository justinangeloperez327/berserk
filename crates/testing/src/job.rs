use framework_jobs::{ErrorKind, Job, JobContext, JobError, Result};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Clone)]
pub struct JobProbe {
    attempts: Arc<Mutex<Vec<u32>>>,
}
impl JobProbe {
    pub fn attempts(&self) -> Vec<u32> {
        self.attempts
            .lock()
            .expect("job probe lock poisoned")
            .clone()
    }
    pub fn count(&self) -> usize {
        self.attempts.lock().expect("job probe lock poisoned").len()
    }
}

pub struct RecordingJob {
    name: &'static str,
    failures_remaining: Arc<AtomicUsize>,
    probe: JobProbe,
}
impl RecordingJob {
    pub fn succeed(name: &'static str) -> (Self, JobProbe) {
        Self::failing(name, 0)
    }
    pub fn failing(name: &'static str, failures: usize) -> (Self, JobProbe) {
        let probe = JobProbe {
            attempts: Arc::new(Mutex::new(Vec::new())),
        };
        (
            Self {
                name,
                failures_remaining: Arc::new(AtomicUsize::new(failures)),
                probe: probe.clone(),
            },
            probe,
        )
    }
}
impl Job for RecordingJob {
    fn name(&self) -> &'static str {
        self.name
    }
    fn handle(&mut self, context: &JobContext) -> Result<()> {
        self.probe
            .attempts
            .lock()
            .map_err(|_| JobError::new(ErrorKind::Handler, "job probe lock poisoned"))?
            .push(context.attempt());
        if self
            .failures_remaining
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok()
        {
            Err(JobError::new(
                ErrorKind::Handler,
                "recording job configured failure",
            ))
        } else {
            Ok(())
        }
    }
}
