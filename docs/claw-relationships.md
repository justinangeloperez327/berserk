# Claw relationships and collections

Claw relationship loaders are batch operations. `HasMany`, `HasOne`, `BelongsTo`, and `BelongsToMany` collect unique parent/foreign keys and query related rows in bounded batches rather than issuing one query per model.

For a normal parent batch below the relationship key chunk size, typed eager loading performs one query for the parent models and one query per eager-loaded relationship. Regression tests count actual `Connection::query` calls so an accidental N+1 implementation fails visibly.

`HasOne` enforces zero-or-one cardinality during batch loading. `BelongsTo` skips NULL foreign keys. Relationship grouping treats equivalent non-negative signed and unsigned database integer values as the same key. Duplicate parent keys do not cause duplicate relationship queries.

`BelongsToMany` preserves actual pivot rows when loading. Pivot uniqueness remains a database constraint; attach/sync helpers must not silently redefine the schema's uniqueness rules.

`Collection<T>` intentionally remains generic and thin. It supports normal slice access, borrowed/mutable/owned iteration, `into_vec`, and small representation-preserving helpers. It does not require `T: Model` and should not grow replacements for the standard iterator API.

Relationship-specific behavior belongs in relationship types, not `Collection<T>`. This keeps collections useful for models, mapped resources, eager-loaded representations, and pagination without hidden database behavior.
