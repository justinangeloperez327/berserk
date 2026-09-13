use crate::{ErrorKind, FailedJob, FailedJobStore, Job, JobContext, JobError, Result};
use std::{
    collections::VecDeque,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::atomic::{AtomicU64, Ordering},
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueueConfig {
    pub workers: usize,
    pub capacity: usize,
}
impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            workers: 1,
            capacity: 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub multiplier: u32,
    pub max_backoff: Duration,
}
impl RetryPolicy {
    pub fn none() -> Self {
        Self {
            max_attempts: 1,
            initial_backoff: Duration::ZERO,
            multiplier: 1,
            max_backoff: Duration::ZERO,
        }
    }
    pub fn validate(self) -> Result<Self> {
        if self.max_attempts == 0 || self.multiplier == 0 {
            return Err(JobError::new(
                ErrorKind::Configuration,
                "retry attempts and multiplier must be greater than zero",
            ));
        }
        if self.initial_backoff > self.max_backoff {
            return Err(JobError::new(
                ErrorKind::Configuration,
                "initial retry backoff cannot exceed the maximum",
            ));
        }
        Ok(self)
    }
    fn delay(self, completed_attempt: u32) -> Duration {
        let exponent = completed_attempt.saturating_sub(1);
        let factor = self.multiplier.saturating_pow(exponent);
        self.initial_backoff
            .checked_mul(factor)
            .unwrap_or(self.max_backoff)
            .min(self.max_backoff)
    }
}
impl Default for RetryPolicy {
    fn default() -> Self {
        Self::none()
    }
}

struct Envelope {
    job: Box<dyn Job>,
    retry: RetryPolicy,
}
struct State {
    pending: VecDeque<Envelope>,
    closed: bool,
}
struct Counters {
    accepted: AtomicU64,
    active: AtomicU64,
    succeeded: AtomicU64,
    failed: AtomicU64,
    retries: AtomicU64,
    panics: AtomicU64,
    failure_store_errors: AtomicU64,
}
impl Default for Counters {
    fn default() -> Self {
        Self {
            accepted: AtomicU64::new(0),
            active: AtomicU64::new(0),
            succeeded: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            retries: AtomicU64::new(0),
            panics: AtomicU64::new(0),
            failure_store_errors: AtomicU64::new(0),
        }
    }
}
struct Shared {
    capacity: usize,
    state: Mutex<State>,
    available: Condvar,
    counters: Counters,
    failures: Arc<dyn FailedJobStore>,
}

#[derive(Clone)]
pub struct JobQueue {
    shared: Arc<Shared>,
}
impl JobQueue {
    pub fn dispatch<J: Job>(&self, job: J, retry: RetryPolicy) -> Result<()> {
        self.dispatch_boxed(Box::new(job), retry)
    }
    pub(crate) fn dispatch_boxed(&self, job: Box<dyn Job>, retry: RetryPolicy) -> Result<()> {
        let retry = retry.validate()?;
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| JobError::new(ErrorKind::Closed, "job queue lock poisoned"))?;
        if state.closed {
            return Err(JobError::new(ErrorKind::Closed, "job queue is closed"));
        }
        if state.pending.len() >= self.shared.capacity {
            return Err(JobError::new(ErrorKind::QueueFull, "job queue is full"));
        }
        state.pending.push_back(Envelope { job, retry });
        self.shared
            .counters
            .accepted
            .fetch_add(1, Ordering::Relaxed);
        drop(state);
        self.shared.available.notify_one();
        Ok(())
    }
    pub fn snapshot(&self) -> Result<QueueSnapshot> {
        let state = self
            .shared
            .state
            .lock()
            .map_err(|_| JobError::new(ErrorKind::Closed, "job queue lock poisoned"))?;
        Ok(QueueSnapshot {
            pending: state.pending.len(),
            active: load(&self.shared.counters.active),
            closed: state.closed,
            accepted: load(&self.shared.counters.accepted),
            succeeded: load(&self.shared.counters.succeeded),
            failed: load(&self.shared.counters.failed),
            retries: load(&self.shared.counters.retries),
            panics: load(&self.shared.counters.panics),
            failure_store_errors: load(&self.shared.counters.failure_store_errors),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QueueSnapshot {
    pub pending: usize,
    pub active: u64,
    pub closed: bool,
    pub accepted: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub retries: u64,
    pub panics: u64,
    pub failure_store_errors: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShutdownReport {
    pub workers_joined: usize,
    pub snapshot: QueueSnapshot,
}

pub struct WorkerPool {
    queue: JobQueue,
    workers: Vec<JoinHandle<()>>,
}
impl WorkerPool {
    pub fn new(config: QueueConfig, failures: Arc<dyn FailedJobStore>) -> Result<Self> {
        if config.workers == 0 || config.capacity == 0 {
            return Err(JobError::new(
                ErrorKind::Configuration,
                "worker and queue capacity must be greater than zero",
            ));
        }
        let shared = Arc::new(Shared {
            capacity: config.capacity,
            state: Mutex::new(State {
                pending: VecDeque::new(),
                closed: false,
            }),
            available: Condvar::new(),
            counters: Counters::default(),
            failures,
        });
        let mut workers = Vec::with_capacity(config.workers);
        for index in 0..config.workers {
            let worker_shared = Arc::clone(&shared);
            let handle = thread::Builder::new()
                .name(format!("framework-job-{index}"))
                .spawn(move || work(worker_shared))
                .map_err(|error| {
                    close(&shared);
                    JobError::new(
                        ErrorKind::Configuration,
                        format!("could not start job worker: {error}"),
                    )
                })?;
            workers.push(handle);
        }
        Ok(Self {
            queue: JobQueue { shared },
            workers,
        })
    }
    pub fn queue(&self) -> JobQueue {
        self.queue.clone()
    }
    pub fn shutdown(mut self) -> Result<ShutdownReport> {
        close(&self.queue.shared);
        let expected = self.workers.len();
        for handle in self.workers.drain(..) {
            handle.join().map_err(|_| {
                JobError::new(
                    ErrorKind::HandlerPanic,
                    "job worker panicked outside its handler boundary",
                )
            })?;
        }
        Ok(ShutdownReport {
            workers_joined: expected,
            snapshot: self.queue.snapshot()?,
        })
    }
}
impl Drop for WorkerPool {
    fn drop(&mut self) {
        close(&self.queue.shared);
    }
}

fn close(shared: &Shared) {
    if let Ok(mut state) = shared.state.lock() {
        state.closed = true;
        shared.available.notify_all();
    }
}
fn work(shared: Arc<Shared>) {
    while let Some(mut envelope) = receive(&shared) {
        shared.counters.active.fetch_add(1, Ordering::Relaxed);
        let name = match catch_unwind(AssertUnwindSafe(|| envelope.job.name())) {
            Ok(name) => name,
            Err(_) => {
                shared.counters.panics.fetch_add(1, Ordering::Relaxed);
                shared.counters.failed.fetch_add(1, Ordering::Relaxed);
                record_failure(
                    &shared,
                    FailedJob {
                        name: "<unknown>",
                        attempts: 0,
                        failed_at: now().unwrap_or(0),
                        error: "job name panicked".into(),
                    },
                );
                shared.counters.active.fetch_sub(1, Ordering::Relaxed);
                continue;
            }
        };
        let mut last_error = None;
        let mut attempts = 0;
        for attempt in 1..=envelope.retry.max_attempts {
            attempts = attempt;
            let result = catch_unwind(AssertUnwindSafe(|| {
                envelope.job.handle(&JobContext { attempt })
            }));
            match result {
                Ok(Ok(())) => {
                    shared.counters.succeeded.fetch_add(1, Ordering::Relaxed);
                    last_error = None;
                    break;
                }
                Ok(Err(error)) => last_error = Some(error.to_string()),
                Err(_) => {
                    shared.counters.panics.fetch_add(1, Ordering::Relaxed);
                    last_error = Some("job handler panicked".into());
                }
            }
            if attempt < envelope.retry.max_attempts {
                shared.counters.retries.fetch_add(1, Ordering::Relaxed);
                let delay = envelope.retry.delay(attempt);
                if !delay.is_zero() {
                    thread::sleep(delay);
                }
            }
        }
        if let Some(error) = last_error {
            shared.counters.failed.fetch_add(1, Ordering::Relaxed);
            let failure = FailedJob {
                name,
                attempts,
                failed_at: now().unwrap_or(0),
                error,
            };
            record_failure(&shared, failure);
        }
        shared.counters.active.fetch_sub(1, Ordering::Relaxed);
    }
}
fn receive(shared: &Shared) -> Option<Envelope> {
    let mut state = shared.state.lock().ok()?;
    loop {
        if let Some(job) = state.pending.pop_front() {
            return Some(job);
        }
        if state.closed {
            return None;
        }
        state = shared.available.wait(state).ok()?;
    }
}
fn load(counter: &AtomicU64) -> u64 {
    counter.load(Ordering::Relaxed)
}
fn record_failure(shared: &Shared, failure: FailedJob) {
    if !matches!(
        catch_unwind(AssertUnwindSafe(|| shared.failures.record(failure))),
        Ok(Ok(()))
    ) {
        shared
            .counters
            .failure_store_errors
            .fetch_add(1, Ordering::Relaxed);
    }
}
fn now() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .map_err(|_| JobError::new(ErrorKind::Clock, "system clock is before Unix epoch"))
}
