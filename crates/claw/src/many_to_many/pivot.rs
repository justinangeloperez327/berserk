use super::BelongsToMany;
use crate::relationship::{keys_equal, validate_key};
use crate::{with_scoped_connection, Model, Transaction};
use berserk_database::{
    scope::with_connection, Connection, DatabaseError, ErrorKind, Execution, Query, Result, Value,
};

/// Key membership changes made by a successful atomic pivot synchronization.
///
/// These are distinct keys, not row counts. Nonnegative integer keys use `U64`.
/// Attached keys follow request order; detached key order is database-dependent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SyncResult {
    pub attached: Vec<Value>,
    pub detached: Vec<Value>,
}

impl<P, R: Model> BelongsToMany<P, R> {
    /// Insert one pivot row. Existing rows are not checked or silently ignored.
    pub fn attach(&self, parent: &P, key: impl Into<Value>) -> Result<Execution> {
        with_connection(|connection| self.attach_on(connection, parent, key))
    }

    pub fn attach_on(
        &self,
        connection: &mut dyn Connection,
        parent: &P,
        key: impl Into<Value>,
    ) -> Result<Execution> {
        let parent = self.valid_parent_key(parent)?;
        let key = key.into();
        validate_key(&key)?;
        self.insert_link(parent, key).execute(connection)
    }

    /// Atomically insert one row per distinct requested key; an empty list is a no-op.
    /// Database uniqueness and foreign-key errors are propagated.
    pub fn attach_many<I, V>(&self, parent: &P, keys: I) -> Result<u64>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        with_connection(|connection| self.attach_many_on(connection, parent, keys))
    }

    pub fn attach_many_on<I, V>(
        &self,
        connection: &mut dyn Connection,
        parent: &P,
        keys: I,
    ) -> Result<u64>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        let parent = self.valid_parent_key(parent)?;
        let keys = distinct_keys(keys)?;
        if keys.is_empty() {
            return Ok(0);
        }
        with_scoped_connection(connection, || {
            Transaction::run(|| {
                with_connection(|connection| {
                    let mut affected = 0_u64;
                    for key in keys {
                        let result = self.insert_link(parent.clone(), key).execute(connection)?;
                        affected = affected.checked_add(result.affected_rows).ok_or_else(|| {
                            DatabaseError::new(ErrorKind::Query, "affected row count overflow")
                        })?;
                    }
                    Ok(affected)
                })
            })
        })
    }

    /// Delete every pivot row matching this parent and related key.
    pub fn detach(&self, parent: &P, key: impl Into<Value>) -> Result<Execution> {
        with_connection(|connection| self.detach_on(connection, parent, key))
    }

    pub fn detach_on(
        &self,
        connection: &mut dyn Connection,
        parent: &P,
        key: impl Into<Value>,
    ) -> Result<Execution> {
        self.detach_many_on(connection, parent, [key])
    }

    /// Delete selected links only. An empty list never means delete all.
    pub fn detach_many<I, V>(&self, parent: &P, keys: I) -> Result<Execution>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        with_connection(|connection| self.detach_many_on(connection, parent, keys))
    }

    pub fn detach_many_on<I, V>(
        &self,
        connection: &mut dyn Connection,
        parent: &P,
        keys: I,
    ) -> Result<Execution>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        let parent = self.valid_parent_key(parent)?;
        let keys = distinct_keys(keys)?;
        if keys.is_empty() {
            return Ok(Execution::default());
        }
        self.links_for(parent)
            .where_in(self.related_pivot_key, keys)
            .delete()
            .execute(connection)
    }

    /// Explicitly remove all links for one parent, leaving other parents untouched.
    pub fn detach_all(&self, parent: &P) -> Result<Execution> {
        with_connection(|connection| self.detach_all_on(connection, parent))
    }

    pub fn detach_all_on(&self, connection: &mut dyn Connection, parent: &P) -> Result<Execution> {
        self.links_for(self.valid_parent_key(parent)?)
            .delete()
            .execute(connection)
    }

    /// Atomically reconcile key membership. Empty input removes all this parent's links.
    /// Retained duplicate rows are preserved. Nested transactions are unsupported.
    pub fn sync<I, V>(&self, parent: &P, keys: I) -> Result<SyncResult>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        with_connection(|connection| self.sync_on(connection, parent, keys))
    }

    pub fn sync_on<I, V>(
        &self,
        connection: &mut dyn Connection,
        parent: &P,
        keys: I,
    ) -> Result<SyncResult>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        let parent = self.valid_parent_key(parent)?;
        let desired = distinct_keys(keys)?;
        with_scoped_connection(connection, || {
            Transaction::run(|| {
                with_connection(|connection| {
                    let rows = self
                        .links_for(parent.clone())
                        .select([self.related_pivot_key])
                        .get(connection)?;
                    let current = distinct_keys(rows.iter().map(|row| {
                        row.get(self.related_pivot_key)
                            .cloned()
                            .unwrap_or(Value::Null)
                    }))
                    .map_err(|_| {
                        DatabaseError::new(
                            ErrorKind::Decode,
                            "pivot row has a missing or invalid related key",
                        )
                    })?;
                    let attached: Vec<_> = desired
                        .iter()
                        .filter(|key| !current.contains(key))
                        .cloned()
                        .collect();
                    let detached: Vec<_> = current
                        .into_iter()
                        .filter(|key| !desired.contains(key))
                        .collect();
                    if !detached.is_empty() {
                        self.links_for(parent.clone())
                            .where_in(self.related_pivot_key, detached.iter().cloned())
                            .delete()
                            .execute(connection)?;
                    }
                    for key in &attached {
                        self.insert_link(parent.clone(), key.clone())
                            .execute(connection)?;
                    }
                    Ok(SyncResult { attached, detached })
                })
            })
        })
    }

    fn valid_parent_key(&self, parent: &P) -> Result<Value> {
        let key = (self.parent_key)(parent);
        validate_key(&key)?;
        Ok(key)
    }

    fn links_for(&self, parent: Value) -> Query {
        Query::table(self.pivot_table).where_(self.foreign_pivot_key, "=", parent)
    }

    fn insert_link(&self, parent: Value, related: Value) -> Query {
        Query::table(self.pivot_table).insert([
            (self.foreign_pivot_key, parent),
            (self.related_pivot_key, related),
        ])
    }
}

fn distinct_keys<I, V>(keys: I) -> Result<Vec<Value>>
where
    I: IntoIterator<Item = V>,
    V: Into<Value>,
{
    let mut distinct = Vec::new();
    for key in keys {
        let key = key.into();
        validate_key(&key)?;
        let key = match key {
            Value::I64(value) if value >= 0 => Value::U64(value as u64),
            key => key,
        };
        if !distinct.iter().any(|seen| keys_equal(seen, &key)) {
            distinct.push(key);
        }
    }
    Ok(distinct)
}
