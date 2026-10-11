use berserk_jobs::{ErrorKind, Job, JobContext, JobError, Result};
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
    pub fn last_attempt(&self) -> Option<u32> {
        self.attempts
            .lock()
            .expect("job probe lock poisoned")
            .last()
            .copied()
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
        let mut remaining = self.failures_remaining.load(Ordering::SeqCst);
        loop {
            if remaining == 0 {
                return Ok(());
            }
            match self.failures_remaining.compare_exchange_weak(
                remaining,
                remaining - 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    return Err(JobError::new(
                        ErrorKind::Handler,
                        "recording job configured failure",
                    ))
                }
                Err(actual) => remaining = actual,
            }
        }
    }
}
