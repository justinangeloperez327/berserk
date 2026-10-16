use berserk_jobs::{
    ErrorKind, Job, JobContext, JobError, MemoryFailedJobs, QueueConfig, RetryPolicy, Scheduler,
    WorkerPool,
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

struct CountJob(Arc<AtomicUsize>);
impl Job for CountJob {
    fn name(&self) -> &'static str {
        "count"
    }
    fn handle(&mut self, _context: &JobContext) -> berserk_jobs::Result<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

struct RetryJob(Arc<AtomicUsize>);
impl Job for RetryJob {
    fn name(&self) -> &'static str {
        "retry"
    }
    fn handle(&mut self, context: &JobContext) -> berserk_jobs::Result<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if context.attempt() < 3 {
            Err(JobError::new(ErrorKind::Handler, "try again"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn workers_retry_then_drain_on_shutdown() {
    let failures = Arc::new(MemoryFailedJobs::new(8).unwrap());
    let pool = WorkerPool::new(
        QueueConfig {
            workers: 2,
            capacity: 8,
        },
        failures.clone(),
    )
    .unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    pool.queue()
        .dispatch(
            RetryJob(Arc::clone(&attempts)),
            RetryPolicy {
                max_attempts: 3,
                initial_backoff: Duration::ZERO,
                multiplier: 2,
                max_backoff: Duration::ZERO,
            },
        )
        .unwrap();
    let report = pool.shutdown().unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert_eq!(report.snapshot.succeeded, 1);
    assert_eq!(report.snapshot.retries, 2);
    assert!(failures.all().unwrap().is_empty());
}

struct FailJob;
impl Job for FailJob {
    fn name(&self) -> &'static str {
        "fail"
    }
    fn handle(&mut self, _context: &JobContext) -> berserk_jobs::Result<()> {
        Err(JobError::new(ErrorKind::Handler, "failed"))
    }
}

#[test]
fn exhausted_jobs_are_recorded_without_payloads() {
    let failures = Arc::new(MemoryFailedJobs::new(8).unwrap());
    let pool = WorkerPool::new(
        QueueConfig {
            workers: 1,
            capacity: 2,
        },
        failures.clone(),
    )
    .unwrap();
    pool.queue()
        .dispatch(
            FailJob,
            RetryPolicy {
                max_attempts: 2,
                initial_backoff: Duration::ZERO,
                multiplier: 1,
                max_backoff: Duration::ZERO,
            },
        )
        .unwrap();
    let report = pool.shutdown().unwrap();
    assert_eq!(report.snapshot.failed, 1);
    let stored = failures.all().unwrap();
    assert_eq!(stored[0].name, "fail");
    assert_eq!(stored[0].attempts, 2);
    assert_eq!(stored[0].error, "failed");
}

struct BlockingJob {
    started: Option<mpsc::Sender<()>>,
    release: mpsc::Receiver<()>,
}
impl Job for BlockingJob {
    fn name(&self) -> &'static str {
        "blocking"
    }
    fn handle(&mut self, _context: &JobContext) -> berserk_jobs::Result<()> {
        self.started.take().unwrap().send(()).unwrap();
        self.release.recv().unwrap();
        Ok(())
    }
}

#[test]
fn queue_capacity_fails_fast() {
    let failures = Arc::new(MemoryFailedJobs::new(8).unwrap());
    let pool = WorkerPool::new(
        QueueConfig {
            workers: 1,
            capacity: 1,
        },
        failures,
    )
    .unwrap();
    let queue = pool.queue();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    queue
        .dispatch(
            BlockingJob {
                started: Some(started_tx),
                release: release_rx,
            },
            RetryPolicy::none(),
        )
        .unwrap();
    started_rx.recv().unwrap();
    queue
        .dispatch(CountJob(Arc::new(AtomicUsize::new(0))), RetryPolicy::none())
        .unwrap();
    assert_eq!(
        queue
            .dispatch(CountJob(Arc::new(AtomicUsize::new(0))), RetryPolicy::none())
            .unwrap_err()
            .kind(),
        ErrorKind::QueueFull
    );
    release_tx.send(()).unwrap();
    pool.shutdown().unwrap();
}

#[test]
fn due_schedules_enqueue_one_job_per_tick() {
    let failures = Arc::new(MemoryFailedJobs::new(8).unwrap());
    let pool = WorkerPool::new(
        QueueConfig {
            workers: 1,
            capacity: 4,
        },
        failures,
    )
    .unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let scheduler = Scheduler::new();
    let shared = Arc::clone(&count);
    scheduler
        .every(Duration::from_millis(1), RetryPolicy::none(), move || {
            CountJob(Arc::clone(&shared))
        })
        .unwrap();
    assert_eq!(
        scheduler
            .run_due_at(&pool.queue(), u64::MAX - 1)
            .unwrap()
            .dispatched,
        1
    );
    pool.shutdown().unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn schedules_reject_intervals_below_clock_precision() {
    let scheduler = Scheduler::new();
    let result = scheduler.every(Duration::from_nanos(1), RetryPolicy::none(), || {
        CountJob(Arc::new(AtomicUsize::new(0)))
    });
    assert_eq!(result.unwrap_err().kind(), ErrorKind::Configuration);
}

#[test]
fn scheduled_factories_can_modify_the_scheduler_without_deadlocking() {
    let scheduler = Arc::new(Scheduler::new());
    let weak = Arc::downgrade(&scheduler);
    let pool = WorkerPool::new(
        QueueConfig::default(),
        Arc::new(MemoryFailedJobs::new(8).unwrap()),
    )
    .unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let shared = Arc::clone(&count);
    scheduler
        .every(Duration::from_millis(1), RetryPolicy::none(), move || {
            let scheduler = weak.upgrade().unwrap();
            let new_id = scheduler
                .every(Duration::from_millis(1), RetryPolicy::none(), || {
                    CountJob(Arc::new(AtomicUsize::new(0)))
                })
                .unwrap();
            assert!(scheduler.forget(new_id).unwrap());
            CountJob(Arc::clone(&shared))
        })
        .unwrap();
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let report = scheduler.run_due_at(&pool.queue(), u64::MAX - 1).unwrap();
        assert_eq!(report.dispatched, 1);
        pool.shutdown().unwrap();
        done_tx.send(()).unwrap();
    });
    done_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("factory must not hold scheduler state lock");
    assert_eq!(count.load(Ordering::SeqCst), 1);
}

#[test]
fn a_panicking_factory_does_not_poison_the_scheduler_or_advance_its_deadline() {
    let scheduler = Scheduler::new();
    let pool = WorkerPool::new(
        QueueConfig::default(),
        Arc::new(MemoryFailedJobs::new(8).unwrap()),
    )
    .unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let shared = Arc::clone(&attempts);
    let id = scheduler
        .every(Duration::from_millis(1), RetryPolicy::none(), move || {
            if shared.fetch_add(1, Ordering::SeqCst) == 0 {
                panic!("factory failed");
            }
            CountJob(Arc::new(AtomicUsize::new(0)))
        })
        .unwrap();
    assert_eq!(
        scheduler
            .run_due_at(&pool.queue(), u64::MAX - 1)
            .unwrap_err()
            .kind(),
        ErrorKind::HandlerPanic
    );
    assert_eq!(
        scheduler
            .run_due_at(&pool.queue(), u64::MAX - 1)
            .unwrap()
            .dispatched,
        1
    );
    assert!(scheduler.forget(id).unwrap());
    pool.shutdown().unwrap();
}
