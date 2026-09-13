# Phase 7 — Reliability

## Implemented changes

Transport checks the total deadline both before and after every read/write. A late read cannot progress into handler execution. Partial writes that finish after the deadline are reported as failures and never retried. Queue wait remains included in the read deadline; response writing has a separate deadline.

Worker connection processing now has an outer unwind boundary so unexpected connection-processing panics do not normally remove a worker. The existing handler boundary remains responsible for generic HTTP 500 responses. Neither boundary works with abort panics, restores shared-state invariants, prevents OOM aborts, or safely handles all pathological panicking destructor behavior. Panic hooks remain Rust defaults.

Server::stats returns cloneable atomic counters: accepted = sockets submitted or rejected at queue handoff; rejected = overload or disconnected queue; completed = response fully written (including HTTP error responses); failed = transport/setup failure or caught connection panic. Snapshots are approximate while work is active and counters can eventually wrap. These expose outcomes without retaining request data or invoking user callbacks. Handler errors rendered as 500 count as completed transport responses, not failed connections.

## New test sources

Idle client expiry and worker recovery; deterministic queue saturation using a gated handler and counter polling; queued request draining after shutdown; truncated request cleanup. Tests use time limits and ephemeral loopback ports. Gate release guards limit stranded work after assertion failure. Existing handler-panic and codec tests remain.

## Review and limits

Compilation and all Rust tests remain unexecuted because Rust tooling is unavailable. Source review and manifest/archive validation are the only completed verification. Reliability source scope is implemented, but runtime reliability is not established. Socket tests, slow-drip total-deadline tests, sustained overload/load testing, write-timeout stress and platform behavior still require execution and further coverage before production use.

Shutdown stops accepting and drains the bounded queue; handlers must return. No forceful handler termination, portable signal handling, production logging or full HTTP conformance claim. Overloaded sockets are closed, not given 503. Generic codec functions need deadline-aware transports. Memory bounds cover incoming configured data, not arbitrary allocations in handlers or responses. Phase 8 consumer verification should resolve compiler/runtime failures before any release decision.
