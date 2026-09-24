# Claw query core contract

Claw separates query construction from terminal execution.

Builder methods such as `where_`, `where_in`, `where_between`, null predicates, `order_by`, `limit`, `offset`, and `scope` only construct a query. Values remain bindings and are never interpolated into SQL text.

Terminal methods execute against the active database scope: `get`, `first`, `first_or_fail`, `count`, `exists`, `update`, `delete`, and `paginate`. Explicit-connection variants retain their `_on` naming.

The query-combination tests lock predicate order, binding order, and driver placeholder behavior for SQLite, MySQL, and PostgreSQL. Empty IN/NOT IN inputs must not corrupt later placeholder numbering.

Model persistence has a separate lifecycle contract: `create` returns the inserted model, `update` refreshes the current instance, `fresh` returns a separate current instance, `refresh` replaces the current instance, and `delete`/`destroy` report affected rows.

Pagination is one-based. `per_page` must be between 1 and 1000 inclusive, page/offset arithmetic must not overflow the supported SQL range, and metadata remains consistent for empty and non-empty result sets.

These contracts are additive to the frozen Berserk 1.0 API. Query-core hardening should prefer regression coverage and internal corrections over new query vocabulary unless an application requirement demonstrates a missing primitive.
