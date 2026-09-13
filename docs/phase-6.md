# Phase 6 — Server execution

## Changes

Server owns a bound nonblocking listener and shared immutable App. run creates the configured worker count using fallible thread builders, then feeds a bounded sync_channel. A receiver mutex protects dequeue only; handlers execute outside that lock concurrently. Startup failure disconnects the queue and joins already-started workers. Shutdown drops the listener before draining accepted work. An already-issued accept can race with a shutdown request, so an immediate post-accept check discards it when observed.

Queue saturation drops the accepted socket rather than blocking acceptance on an error response. Resource scope is configured workers plus queued sockets and the listener's transient accepted socket; the OS backlog is separate. No thread per connection. A 5 ms nonblocking accept poll permits shutdown checks without a platform-specific wake primitive.

The transport applies min(idle timeout, remaining deadline) to each blocking read/write. Request deadline starts at acceptance, including queue wait. Header parsing remains bytewise and therefore potentially syscall-heavy; performance optimization is deferred. Each response has its own total write deadline. Socket timeout granularity is OS-dependent. Synchronous user handlers are not interrupted and can stall draining forever.

Parser I/O failures close silently; syntax/protocol failures receive an empty error response. Parsed HEAD retains method during error conversion. Invalid UTF-8 handler errors become 400; other handler errors and unwind panics become generic 500. Catching a panic does not restore application invariants. Default Rust panic hook still runs. Connection write failures are not retried. Structured diagnostics are deferred.

## Verification status

Two new loopback test sources cover valid/missing routes, panic recovery, continued serving, connection close, bind conflict and shutdown before run. Earlier test sources retained. Compilation and all Rust tests remain unexecuted. Source/manifests reviewed, dependency paths and archive verified. No runtime claims.

## Phase 7 work

Expand overload and slow-client tests, concurrent shutdown/disconnect coverage, resource cleanup review, and failure diagnostics. Document assumptions around handlers, timeout granularity and panic behavior. Network and protocol hardening are not complete merely because the server path is implemented.
