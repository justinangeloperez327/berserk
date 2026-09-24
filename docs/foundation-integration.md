# Foundation integration contract

Foundation integrates Berserk services through explicit application boundaries rather than globals or hidden hooks.

The root application owns cache, object storage, event bus, job queue/worker pool, notification dispatcher, authentication, and database state. Request handlers retrieve only the services needed for the workflow.

The task integration flow demonstrates cache invalidation, typed event dispatch, and bounded background work. Background project snapshots write through the Storage contract. Notification composition is kept as an explicit application operation so delivery failures remain visible to the caller instead of being silently swallowed.

Database mutation and external side effects are intentionally distinct consistency domains. A database transaction cannot atomically commit an email, webhook, file write, or in-memory job enqueue. Foundation therefore does not pretend these operations form a distributed transaction. Durable production delivery should use an outbox/idempotency strategy when database state and external effects must survive process failure together.

Worker ownership remains attached to application state so dropping the application closes worker infrastructure normally; the example does not leak worker threads to extend lifetime.

Service errors are translated at the application boundary. Secrets and notification addresses are not added to request logs. Cache keys and storage paths use stable internal identifiers.

Group 10 is an integration reference, not a requirement that every Berserk application register every service.
