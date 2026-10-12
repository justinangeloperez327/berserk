# Foundation domain

The Foundation example demonstrates a full application rather than isolated HTTP routes.

Its domain now contains users, roles, projects, tasks, and comments. Projects belong to an owner, tasks belong to a project and may have an assignee, and comments belong to a task and author. Database foreign keys encode the same ownership graph as Claw relationships.

The read API demonstrates model route binding, authorization abilities, relationship loading, nested resources, deterministic ordering, filtering, and pagination. Existing user CRUD continues to demonstrate sanitation, request authorization, semantic validation, policy authorization, transactions, many-to-many roles, and Axe rendering.

The application keeps domain models separate from the services that act on them. Events, jobs, notifications, cache, storage, and failure handling are demonstrated in the [Foundation integration guide](foundation-integration.md).

Foundation code should remain understandable as application code. It must not introduce framework-only shortcuts solely to make the example smaller.
