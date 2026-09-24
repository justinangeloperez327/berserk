# Foundation domain

The Foundation example is Berserk's realistic application contract rather than a feature showcase made of isolated routes.

Its domain now contains users, roles, projects, tasks, and comments. Projects belong to an owner, tasks belong to a project and may have an assignee, and comments belong to a task and author. Database foreign keys encode the same ownership graph as Claw relationships.

The read API demonstrates model route binding, authorization abilities, relationship loading, nested resources, deterministic ordering, filtering, and pagination. Existing user CRUD continues to demonstrate sanitation, request authorization, semantic validation, policy authorization, transactions, many-to-many roles, and Axe rendering.

Group 9 deliberately expands the domain before adding every infrastructure integration. Events, jobs, notifications, cache, storage, broader mutation workflows, and failure-path orchestration belong to the Foundation integration group so the domain model remains reviewable.

Foundation code should remain understandable as application code. It must not introduce framework-only shortcuts solely to make the example smaller.
