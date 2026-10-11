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

The live database workflow runs bundled SQLite, PostgreSQL 18, and MySQL 26.7 on relevant pull requests. These baselines are defined in [support-policy.md](support-policy.md). MongoDB 8.3.11 receives a server-version smoke check only; Berserk has no MongoDB driver.

Use `BERSERK_POSTGRES_TEST_URL` and `BERSERK_MYSQL_TEST_URL` for dedicated integration-test databases. The older `FRAMEWORK_POSTGRES_TEST_URL` and `FRAMEWORK_MYSQL_TEST_URL` names remain fallback aliases. Tests create and drop their own contract tables; never point these variables at an application database. When `BERSERK_REQUIRE_LIVE_DATABASES=1`, missing live database URLs fail rather than silently skipping the contract.

PostgreSQL bindings use the prepared statement's inferred types. NULL bindings are supported, integers are checked against SMALLINT/INTEGER/BIGINT/OID widths, and REAL/DOUBLE PRECISION use their matching encoders. Out-of-range values and incompatible non-NULL types fail instead of being silently coerced.
