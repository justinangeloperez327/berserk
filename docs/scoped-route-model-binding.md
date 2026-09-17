# Scoped Route Model Binding

Berserk scopes nested Claw model binding to the parent model by default.

For a route such as:

```rust
route.get(
    "/users/{user}/posts/{post}",
    PostController::show,
)?;
```

with a controller:

```rust
fn show(user: User, post: Post) -> Result<Response> {
    // `post` has already been resolved within `user`.
    todo!()
}
```

`Post` must explicitly declare how it belongs to `User`:

```rust
use framework::claw::ScopedRouteModel;

impl ScopedRouteModel<User> for Post {
    const PARENT_FOREIGN_KEY: &'static str = "user_id";
}
```

The default scoped lookup is equivalent to:

```rust
Post::query()
    .where_(Post::PRIMARY_KEY, "=", post_key)
    .where_("user_id", "=", user.key())
    .first(&mut connection)?;
```

Therefore `/users/7/posts/3` binds only when post `3` belongs to user `7`. If post `3` exists under another user, Berserk returns `404 Not Found` instead of exposing that record through the nested route.

Both route keys are parsed before database acquisition. The parent and child are then resolved through one acquired connection. A malformed key returns `400 Bad Request`; a missing parent, missing child, or child outside the parent scope returns `404 Not Found`.

## Custom scopes

When the relationship cannot be represented by one foreign key, override `scoped_route_query`:

```rust
impl ScopedRouteModel<Account> for Membership {
    const PARENT_FOREIGN_KEY: &'static str = "account_id";

    fn scoped_route_query(
        account: &Account,
        key: Value,
    ) -> ModelQuery<Self> {
        Self::query()
            .where_(Self::PRIMARY_KEY, "=", key)
            .where_("account_id", "=", account.key())
            .where_("active", "=", true)
    }
}
```

`PARENT_FOREIGN_KEY` remains required so the common ownership relationship is explicit even when the query is customized.

## Unrelated route parameters

A two-model controller signature is treated as a parent→child relationship. If two route values are unrelated resources, use typed route IDs and perform the independent lookups explicitly rather than implementing a fake ownership relationship.
