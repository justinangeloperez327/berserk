# Database backend contract

Berserk supports SQLite, PostgreSQL, and MySQL behind the same driver-neutral `Connection` contract.

The live contract in `crates/database/tests/common` is executed unchanged against every supported backend. It verifies the semantics application and Claw code may rely on rather than requiring byte-for-byte identical vendor behavior.

The contract currently covers:

- bound inserts and reads;
- NULL and Unicode round trips;
- deterministic ordering, limit, and offset;
- unique, foreign-key, and not-null classification;
- rollback visibility;
- migration apply, idempotence, rollback, and failed-migration behavior.

Backend-specific tests remain appropriate for capabilities that cannot be portable, such as transactional DDL or read-only transaction support. Driver-native integer representations may differ between signed and unsigned values when they represent the same non-negative database value.

Applications must branch on Berserk `ErrorKind` rather than PostgreSQL SQLSTATE, MySQL numeric codes, SQLite extended codes, or driver message text.

The live database workflow runs SQLite on every relevant pull request, PostgreSQL 15–18, and MySQL 8.4. When `BERSERK_REQUIRE_LIVE_DATABASES=1`, missing live database URLs fail rather than silently skipping the contract.
