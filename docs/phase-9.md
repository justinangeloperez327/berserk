# Phase 9 — Middleware and state

## Implemented API

App::middleware accepts a Middleware implementation or a synchronous closure taking Request and Next and returning Result<Response>. Registration order runs outermost-first; responses unwind in reverse. Next is consumed and not Clone, permitting at most one continuation. Early responses skip the remaining chain. Errors propagate unless a layer handles them. User code remains subject to existing panic and threading limits.

App::state registers one shared value per type. Duplicate registrations fail without replacing the value. Request::state<T>() returns Option<&T>. Values must be Send + Sync + static; explicit locks/atomics support mutation. Newtypes distinguish multiple values with identical underlying types. State is root-scoped and attached before global middleware. Group state registration is rejected, not silently discarded.

App::group(prefix, closure) builds routes in a temporary App, mounts atomically on success, and attaches its middleware to selected handlers. Prefix root/empty adds nothing; other prefixes join directly, so /api plus / yields /api/. Nested groups work. Group middleware sees selected route params; global middleware runs before routing and cannot see them before calling Next. Group middleware does NOT run for 404 or 405, while global middleware does. Group configuration is not a server configuration scope; only its routes and middleware are mounted. Route conflicts leave the parent unchanged.

RequestId is opt-in middleware. It sets Request::request_id and x-request-id on successful downstream responses, including returned 404/405. Incoming IDs are ignored. It is a monotonically incremented process-local u64 counter, can wrap, is predictable and not globally unique. Error propagation and server-generated 500s outside the chain do not get this response header; no guarantee is made otherwise. Register it first to cover other layers' early responses.

HEAD suppression now happens at the end of App::handle, after global and group middleware. Final validation also checks middleware-created responses. Raw Router dispatch is private.

## Deliverables and verification

Added middleware/state modules, group mounting, request context access, a runnable-intended middleware example and four test functions covering ordering, short circuit, HEAD, state, IDs, group isolation/nesting and rollback. Existing sources retained. Source/manifests and archive reviewed. Compilation, Rust tests and prior Phase 8 runtime gate remain unverified. No new dependencies. No publication.
