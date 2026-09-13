# Phase 23 — Events and Background Jobs

Phase 23 adds two optional crates that remain independent of HTTP and database code.

## Events

`framework-events` provides a typed `EventBus`. Applications implement `Event` with a stable diagnostic name, register typed closures, and dispatch an event by reference. Listeners run synchronously in registration order. Registration returns a `ListenerId` for explicit removal.

Dispatch copies the current listener list before execution, so listeners can be registered or removed while another dispatch is running without holding the bus lock across application code. It stops at the first returned error. A panic is converted into a dispatch error when the binary uses panic unwinding; aborting panic profiles cannot be contained.

Events are deliberately not background jobs. Dispatch completing means all selected listeners completed; no hidden thread or remote call is implied.

## Background jobs

`framework-jobs` provides `WorkerPool`, a clonable `JobQueue`, and a `Job` trait. Queue and worker capacities are fixed and validated. Dispatch is nonblocking: a full or closed queue returns an explicit error. Workers catch handler panics, apply a validated `RetryPolicy`, and write exhausted failures to a `FailedJobStore`.

Retries execute the same owned job instance. Applications must make retryable side effects idempotent because a failure may occur after an external effect succeeded. Backoff occupies one worker, which keeps concurrency bounded but means long retry delays require deliberate worker sizing.

`MemoryFailedJobs` is bounded and evicts its oldest metadata record at capacity. Records contain the job name, attempt count, time, and final error text—not the job payload. Applications must avoid putting secrets in error messages.

Calling `WorkerPool::shutdown` closes the queue, drains accepted jobs including their retries, joins workers, and returns a final snapshot. Dropping a pool requests closure but cannot wait; explicit shutdown is the completion boundary.

## Scheduling

`Scheduler::every` registers a recurring job factory and retry policy. `run_due` is an explicit tick suitable for an application timer. Each due schedule emits at most one job per tick and sets its next deadline relative to that tick, avoiding an unbounded catch-up burst after downtime. A queue-full error leaves the schedule due for a later tick.

## Delivery guarantee

The built-in queue is process-local and in-memory. It drains during controlled shutdown but loses pending work on process or machine failure. It does not claim durable, distributed, at-least-once, or exactly-once delivery. A future Redis, database, or broker adapter must state its acknowledgement, reservation, visibility-timeout, retry, and duplicate-delivery behavior explicitly.

## Feature selection

```toml
framework = { path = "crates/framework", features = ["events", "jobs"] }
```

Applications may instead depend directly on `framework-events` or `framework-jobs`.

## Verification status

Contract tests cover listener order, removal, errors, panic containment, job retries, failed-job recording, queue saturation, draining, and recurring schedules. Rust tooling is unavailable in the preparation environment, so compilation and test execution remain pending.
