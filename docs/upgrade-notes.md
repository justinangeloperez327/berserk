# Upgrade notes

## v0.4.0 → v0.5.0

v0.5.0 adds explicit application-service registration without replacing generic typed state.

| Before | v0.5.0 |
| --- | --- |
| `app.state(cache)?` plus manual typed retrieval | `app.cache(cache)?` and `request.cache()?` |
| Manual storage state wiring | `app.storage(storage)?` and `request.storage()?` |
| Manual event-bus state wiring | `app.events(events)?` and `request.events()?` |
| Manual job-queue state wiring | `app.jobs(queue)?` and `request.jobs()?` |
| Manual outbound-client state wiring | `app.http_client(client)?` and `request.http_client()?` |
| Manual notifier state wiring | `app.notifications(notifier)?` and `request.notifications()?` |

The generic state APIs remain supported, so custom services do not need to adopt a framework-specific container.

## v0.3.0 → v0.4.0

v0.4.0 introduces the authentication/security baseline. Configure a guard once with `app.auth(guard)?`, then use route scopes such as `.auth()`, `.guest()`, and `.can("posts.update")`. Controllers can use `request.user()`, `request.can(...)`, and typed policy authorization.

CORS and security headers are explicit middleware. HSTS remains opt-in and deployment-specific. Browser cookie authentication and CSRF policy are not first-class framework features in v0.4.0.

## v0.2.0 → v0.3.0

v0.3.0 focuses on developer experience while preserving Berserk's instance-based application architecture, request-scoped state, Claw ORM model, and sync-first design.

| v0.2.0 usage | v0.3.0 usage |
| --- | --- |
| `request.json()` for a raw value | `request.json_value()` or `request.json::<Json>()` |
| `request.query()` for all pairs | `request.query_pairs()` |
| Manual lookup in query pairs | `request.query("page")?` |
| `Ok(response().text("OK"))` | `response().text("OK")` |
| `Ok(response().no_content())` | `response().no_content()` |
| `response().json(&json)?.status(201)` | `response().status(201).json(json)?` |

Response-factory terminals return `Result<Response>` so the completed response can be validated. Direct `Response` constructors remain available where documented.

`request.validate::<T>()` is the concise FormRequest entry point. Existing typed FormRequest extraction remains available. Typed application configuration is registered with `App::configure` and accessed with `Request::config`; shared services can be accessed through request state/shared handles.

Network serving is controlled by the `server` feature. Consumers that disable default features must explicitly enable `server` if they call `bind` or `listen`. Async application actions remain independently optional.

The CLI/application skeleton now exposes controllers, models, requests, middleware, configuration, and routes explicitly. See [v0.3.0.md](v0.3.0.md) for the detailed v0.3 API.

## v0.1.0 → v0.2.0

v0.2.0 introduced request-scoped Claw operations and retained explicit connection escape hatches:

| Explicit connection API | Request-scoped API | Explicit escape hatch |
| --- | --- | --- |
| `User::find(connection, key)` | `User::find(key)` | `User::find_on(connection, key)` |
| `query.get(connection)` | `query.get()` | `query.get_on(connection)` |
| `user.save(connection)` | `user.save()` | `user.save_on(connection)` |
| `User::create(connection, columns)` | `User::create(input)` | `User::create_on(connection, columns)` |
| `user.update(connection, columns)` | `user.update(input)` | `user.update_on(connection, columns)` |
| `user.delete(connection)` | `user.delete()` | `user.delete_on(connection)` |
| `query.paginate(connection, page, size)` | `query.paginate(size)` | `query.paginate_on(connection, page, size)` |
| `where_(column, operator, value)` | `where_(column, value)` | `where_op(column, operator, value)` |

FormRequest gained direct input parameters and authorization after validation. `Validated<T>` remained supported. Application errors, including router and authentication errors, moved toward the unified JSON error surface. Optional async actions were added behind the appropriate feature.

## Pre-release consumers

Berserk is still below 1.0. Applications consuming git/path snapshots should treat the documented API for the selected release as the source of truth rather than examples copied from older commits.

Rust 1.88 remains the MSRV for the current baseline. Review [compatibility.md](compatibility.md), [known-limitations.md](known-limitations.md), and the release-specific guide before upgrading.
