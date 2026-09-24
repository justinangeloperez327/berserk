# Transaction integrity

Berserk transactions are synchronous, connection-scoped units of work.

## Guarantees

A successful transaction commits all writes before returning its value. An application or database error triggers rollback. If rollback itself fails, the rollback error is returned because the connection can no longer prove that the original unit of work was reverted.

After commit or rollback, the underlying connection is released back to its enclosing scope and remains usable unless the driver reports a connection failure.

Claw `Transaction::run` and `Transaction::with_options` install the transaction as the active scoped connection. Claw terminal operations therefore join the transaction without receiving a connection parameter.

## Nesting

Nested transactions and savepoints are not part of the Berserk 1.0 transaction contract. An attempt to begin a transaction from an active transaction returns `ErrorKind::Transaction` before the nested operation executes.

Relationship helpers that own their own transaction must reject nesting before mutating rows. Single-write helpers may participate in an existing outer transaction when they use the active scope.

## Cross-backend contract

The same live transaction contract runs against SQLite, PostgreSQL, and MySQL. It verifies commit visibility, explicit rollback, multi-write atomicity, constraint-failure rollback, and connection reuse after transaction completion.

Isolation and read-only capabilities remain backend-specific unless explicitly documented. Code should not assume a database isolation level that Berserk has not requested or guaranteed.
