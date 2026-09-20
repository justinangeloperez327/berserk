use super::dialect;
use crate::{
    Connection, DatabaseError, Driver, ErrorKind, Execution, Result, Row, Statement, Value,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Boolean {
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq)]
enum Predicate {
    Compare {
        boolean: Boolean,
        column: String,
        operator: String,
        value: Value,
    },
    In {
        boolean: Boolean,
        column: String,
        values: Vec<Value>,
        negated: bool,
    },
    Between {
        boolean: Boolean,
        column: String,
        lower: Value,
        upper: Value,
        negated: bool,
    },
    Null {
        boolean: Boolean,
        column: String,
        negated: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct Join {
    kind: JoinKind,
    table: String,
    left: String,
    right: String,
}

#[derive(Clone, Debug, PartialEq)]
enum Operation {
    Select,
    Insert(Vec<(String, Value)>),
    Update(Vec<(String, Value)>),
    Delete,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Builder {
    table: String,
    columns: Vec<String>,
    joins: Vec<Join>,
    predicates: Vec<Predicate>,
    orders: Vec<(String, Direction)>,
    limit: Option<u64>,
    offset: Option<u64>,
    operation: Operation,
    allow_all: bool,
}

impl Builder {
    pub fn raw(sql: impl Into<String>) -> RawQuery {
        RawQuery::new(sql)
    }

    pub fn table(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: vec!["*".into()],
            joins: Vec::new(),
            predicates: Vec::new(),
            orders: Vec::new(),
            limit: None,
            offset: None,
            operation: Operation::Select,
            allow_all: false,
        }
    }

    pub fn select<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.columns = columns.into_iter().map(Into::into).collect();
        self
    }

    pub fn where_(
        self,
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> Self {
        self.compare(Boolean::And, column.into(), operator.into(), value.into())
    }
    pub fn or_where(
        self,
        column: impl Into<String>,
        operator: impl Into<String>,
        value: impl Into<Value>,
    ) -> Self {
        self.compare(Boolean::Or, column.into(), operator.into(), value.into())
    }
    fn compare(mut self, boolean: Boolean, column: String, operator: String, value: Value) -> Self {
        self.predicates.push(Predicate::Compare {
            boolean,
            column,
            operator,
            value,
        });
        self
    }

    pub fn where_in<I, V>(self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.in_list(Boolean::And, column.into(), values, false)
    }
    pub fn or_where_in<I, V>(self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.in_list(Boolean::Or, column.into(), values, false)
    }
    pub fn where_not_in<I, V>(self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.in_list(Boolean::And, column.into(), values, true)
    }
    pub fn or_where_not_in<I, V>(self, column: impl Into<String>, values: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.in_list(Boolean::Or, column.into(), values, true)
    }
    fn in_list<I, V>(mut self, boolean: Boolean, column: String, values: I, negated: bool) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        self.predicates.push(Predicate::In {
            boolean,
            column,
            values: values.into_iter().map(Into::into).collect(),
            negated,
        });
        self
    }

    pub fn where_between(
        self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.between(
            Boolean::And,
            column.into(),
            lower.into(),
            upper.into(),
            false,
        )
    }
    pub fn or_where_between(
        self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.between(
            Boolean::Or,
            column.into(),
            lower.into(),
            upper.into(),
            false,
        )
    }
    pub fn where_not_between(
        self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.between(
            Boolean::And,
            column.into(),
            lower.into(),
            upper.into(),
            true,
        )
    }
    pub fn or_where_not_between(
        self,
        column: impl Into<String>,
        lower: impl Into<Value>,
        upper: impl Into<Value>,
    ) -> Self {
        self.between(Boolean::Or, column.into(), lower.into(), upper.into(), true)
    }
    fn between(
        mut self,
        boolean: Boolean,
        column: String,
        lower: Value,
        upper: Value,
        negated: bool,
    ) -> Self {
        self.predicates.push(Predicate::Between {
            boolean,
            column,
            lower,
            upper,
            negated,
        });
        self
    }

    pub fn where_null(mut self, column: impl Into<String>) -> Self {
        self.predicates.push(Predicate::Null {
            boolean: Boolean::And,
            column: column.into(),
            negated: false,
        });
        self
    }
    pub fn where_not_null(mut self, column: impl Into<String>) -> Self {
        self.predicates.push(Predicate::Null {
            boolean: Boolean::And,
            column: column.into(),
            negated: true,
        });
        self
    }
    pub fn or_where_null(mut self, column: impl Into<String>) -> Self {
        self.predicates.push(Predicate::Null {
            boolean: Boolean::Or,
            column: column.into(),
            negated: false,
        });
        self
    }
    pub fn or_where_not_null(mut self, column: impl Into<String>) -> Self {
        self.predicates.push(Predicate::Null {
            boolean: Boolean::Or,
            column: column.into(),
            negated: true,
        });
        self
    }

    pub fn join(
        self,
        table: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        self.join_as(JoinKind::Inner, table, left, right)
    }
    pub fn left_join(
        self,
        table: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        self.join_as(JoinKind::Left, table, left, right)
    }
    pub fn right_join(
        self,
        table: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        self.join_as(JoinKind::Right, table, left, right)
    }
    fn join_as(
        mut self,
        kind: JoinKind,
        table: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        self.joins.push(Join {
            kind,
            table: table.into(),
            left: left.into(),
            right: right.into(),
        });
        self
    }

    pub fn order_by(mut self, column: impl Into<String>, direction: Direction) -> Self {
        self.orders.push((column.into(), direction));
        self
    }
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn insert<I, S, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = (S, V)>,
        S: Into<String>,
        V: Into<Value>,
    {
        self.operation = Operation::Insert(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        );
        self
    }
    pub fn update<I, S, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = (S, V)>,
        S: Into<String>,
        V: Into<Value>,
    {
        self.operation = Operation::Update(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        );
        self
    }
    pub fn delete(mut self) -> Self {
        self.operation = Operation::Delete;
        self
    }
    pub fn allow_all(mut self) -> Self {
        self.allow_all = true;
        self
    }

    pub fn to_statement(&self, driver: Driver) -> Result<Statement> {
        match &self.operation {
            Operation::Select => self.compile_select(driver),
            Operation::Insert(values) => self.compile_insert(driver, values),
            Operation::Update(values) => self.compile_update(driver, values),
            Operation::Delete => self.compile_delete(driver),
        }
    }

    pub fn get(&self, connection: &mut dyn Connection) -> Result<Vec<Row>> {
        if !matches!(&self.operation, Operation::Select) {
            return Err(query_error(
                "use execute to run insert, update, or delete queries",
            ));
        }
        let driver = connection.driver();
        let statement = self.to_statement(driver)?;
        connection.query(&statement)
    }
    pub fn count(&self, connection: &mut dyn Connection) -> Result<u64> {
        if !matches!(&self.operation, Operation::Select) {
            return Err(query_error("count can only execute a select query"));
        }
        let statement = self.compile_count(connection.driver())?;
        let row = connection
            .query(&statement)?
            .into_iter()
            .next()
            .ok_or_else(|| query_error("count query returned no row"))?;
        match row.get("aggregate") {
            Some(Value::U64(value)) => Ok(*value),
            Some(Value::I64(value)) => u64::try_from(*value)
                .map_err(|_| query_error("count query returned a negative value")),
            _ => Err(DatabaseError::new(
                ErrorKind::Decode,
                "count query did not return an integer `aggregate` column",
            )),
        }
    }
    pub fn first(mut self, connection: &mut dyn Connection) -> Result<Option<Row>> {
        self.limit = Some(1);
        Ok(self.get(connection)?.into_iter().next())
    }
    pub fn execute(&self, connection: &mut dyn Connection) -> Result<Execution> {
        if matches!(&self.operation, Operation::Select) {
            return Err(DatabaseError::new(
                ErrorKind::Query,
                "use get or first to execute a select query",
            ));
        }
        let driver = connection.driver();
        let statement = self.to_statement(driver)?;
        connection.execute(&statement)
    }

    fn compile_select(&self, driver: Driver) -> Result<Statement> {
        if self.columns.is_empty() {
            return Err(query_error("select requires at least one column"));
        }
        let columns = self
            .columns
            .iter()
            .map(|column| dialect::identifier(driver, column))
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        let mut sql = format!(
            "SELECT {columns} FROM {}",
            dialect::identifier(driver, &self.table)?
        );
        self.compile_joins(driver, &mut sql)?;
        let mut bindings = Vec::new();
        self.compile_predicates(driver, &mut sql, &mut bindings)?;
        self.compile_order_limit(driver, &mut sql)?;
        Ok(statement(sql, bindings))
    }

    fn compile_count(&self, driver: Driver) -> Result<Statement> {
        let mut sql = format!(
            "SELECT COUNT(*) AS {} FROM {}",
            dialect::identifier(driver, "aggregate")?,
            dialect::identifier(driver, &self.table)?
        );
        self.compile_joins(driver, &mut sql)?;
        let mut bindings = Vec::new();
        self.compile_predicates(driver, &mut sql, &mut bindings)?;
        Ok(statement(sql, bindings))
    }

    fn compile_insert(&self, driver: Driver, values: &[(String, Value)]) -> Result<Statement> {
        if values.is_empty() {
            return Err(query_error("insert requires at least one value"));
        }
        unique_columns(values)?;
        if !self.predicates.is_empty()
            || !self.joins.is_empty()
            || !self.orders.is_empty()
            || self.limit.is_some()
            || self.offset.is_some()
        {
            return Err(query_error(
                "insert cannot contain filters, joins, ordering, limit, or offset",
            ));
        }
        let columns = values
            .iter()
            .map(|(column, _)| dialect::identifier(driver, column))
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        let placeholders = (1..=values.len())
            .map(|index| dialect::placeholder(driver, index))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(statement(
            format!(
                "INSERT INTO {} ({columns}) VALUES ({placeholders})",
                dialect::identifier(driver, &self.table)?
            ),
            values.iter().map(|(_, value)| value.clone()).collect(),
        ))
    }

    fn compile_update(&self, driver: Driver, values: &[(String, Value)]) -> Result<Statement> {
        if values.is_empty() {
            return Err(query_error("update requires at least one value"));
        }
        unique_columns(values)?;
        self.require_safe_mutation()?;
        if !self.joins.is_empty()
            || !self.orders.is_empty()
            || self.limit.is_some()
            || self.offset.is_some()
        {
            return Err(query_error(
                "portable updates cannot contain joins, ordering, limit, or offset",
            ));
        }
        let mut bindings: Vec<Value> = values.iter().map(|(_, value)| value.clone()).collect();
        let assignments = values
            .iter()
            .enumerate()
            .map(|(index, (column, _))| {
                Ok(format!(
                    "{} = {}",
                    dialect::identifier(driver, column)?,
                    dialect::placeholder(driver, index + 1)
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .join(", ");
        let mut sql = format!(
            "UPDATE {} SET {assignments}",
            dialect::identifier(driver, &self.table)?
        );
        self.compile_predicates(driver, &mut sql, &mut bindings)?;
        Ok(statement(sql, bindings))
    }

    fn compile_delete(&self, driver: Driver) -> Result<Statement> {
        self.require_safe_mutation()?;
        if !self.joins.is_empty()
            || !self.orders.is_empty()
            || self.limit.is_some()
            || self.offset.is_some()
        {
            return Err(query_error(
                "portable deletes cannot contain joins, ordering, limit, or offset",
            ));
        }
        let mut sql = format!("DELETE FROM {}", dialect::identifier(driver, &self.table)?);
        let mut bindings = Vec::new();
        self.compile_predicates(driver, &mut sql, &mut bindings)?;
        Ok(statement(sql, bindings))
    }

    fn require_safe_mutation(&self) -> Result<()> {
        if self.predicates.is_empty() && !self.allow_all {
            Err(query_error(
                "update or delete without filters requires allow_all",
            ))
        } else {
            Ok(())
        }
    }

    fn compile_joins(&self, driver: Driver, sql: &mut String) -> Result<()> {
        for join in &self.joins {
            let kind = match join.kind {
                JoinKind::Inner => "INNER",
                JoinKind::Left => "LEFT",
                JoinKind::Right => "RIGHT",
            };
            sql.push_str(&format!(
                " {kind} JOIN {} ON {} = {}",
                dialect::identifier(driver, &join.table)?,
                dialect::identifier(driver, &join.left)?,
                dialect::identifier(driver, &join.right)?
            ));
        }
        Ok(())
    }

    fn compile_predicates(
        &self,
        driver: Driver,
        sql: &mut String,
        bindings: &mut Vec<Value>,
    ) -> Result<()> {
        if self.predicates.is_empty() {
            return Ok(());
        }
        sql.push_str(" WHERE ");
        for (index, predicate) in self.predicates.iter().enumerate() {
            let boolean = match predicate {
                Predicate::Compare { boolean, .. }
                | Predicate::In { boolean, .. }
                | Predicate::Between { boolean, .. }
                | Predicate::Null { boolean, .. } => *boolean,
            };
            if index > 0 {
                sql.push_str(match boolean {
                    Boolean::And => " AND ",
                    Boolean::Or => " OR ",
                });
            }
            match predicate {
                Predicate::Compare {
                    column,
                    operator,
                    value,
                    ..
                } => {
                    if matches!(value, Value::Null) {
                        return Err(query_error(
                            "use where_null or where_not_null for NULL comparisons",
                        ));
                    }
                    bindings.push(value.clone());
                    sql.push_str(&format!(
                        "{} {} {}",
                        dialect::identifier(driver, column)?,
                        dialect::operator(operator)?,
                        dialect::placeholder(driver, bindings.len())
                    ));
                }
                Predicate::In {
                    column,
                    values,
                    negated,
                    ..
                } => {
                    let column = dialect::identifier(driver, column)?;
                    if values.is_empty() {
                        sql.push_str(if *negated { "1 = 1" } else { "1 = 0" });
                        continue;
                    }
                    let mut placeholders = Vec::with_capacity(values.len());
                    for value in values {
                        if matches!(value, Value::Null) {
                            return Err(query_error("use where_null or where_not_null instead of NULL inside an IN list"));
                        }
                        bindings.push(value.clone());
                        placeholders.push(dialect::placeholder(driver, bindings.len()));
                    }
                    sql.push_str(&format!(
                        "{column} {} ({})",
                        if *negated { "NOT IN" } else { "IN" },
                        placeholders.join(", ")
                    ));
                }
                Predicate::Between {
                    column,
                    lower,
                    upper,
                    negated,
                    ..
                } => {
                    if matches!(lower, Value::Null) || matches!(upper, Value::Null) {
                        return Err(query_error(
                            "BETWEEN bounds cannot be NULL; use explicit NULL predicates",
                        ));
                    }
                    bindings.push(lower.clone());
                    let lower = dialect::placeholder(driver, bindings.len());
                    bindings.push(upper.clone());
                    let upper = dialect::placeholder(driver, bindings.len());
                    sql.push_str(&format!(
                        "{} {}BETWEEN {lower} AND {upper}",
                        dialect::identifier(driver, column)?,
                        if *negated { "NOT " } else { "" }
                    ));
                }
                Predicate::Null {
                    column, negated, ..
                } => {
                    sql.push_str(&format!(
                        "{} IS {}NULL",
                        dialect::identifier(driver, column)?,
                        if *negated { "NOT " } else { "" }
                    ));
                }
            }
        }
        Ok(())
    }

    fn compile_order_limit(&self, driver: Driver, sql: &mut String) -> Result<()> {
        if !self.orders.is_empty() {
            let values = self
                .orders
                .iter()
                .map(|(column, direction)| {
                    Ok(format!(
                        "{} {}",
                        dialect::identifier(driver, column)?,
                        match direction {
                            Direction::Asc => "ASC",
                            Direction::Desc => "DESC",
                        }
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            sql.push_str(" ORDER BY ");
            sql.push_str(&values.join(", "));
        }
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        if let Some(offset) = self.offset {
            if self.limit.is_none() && driver == Driver::MySql {
                sql.push_str(" LIMIT 18446744073709551615");
            } else if self.limit.is_none() && driver == Driver::Sqlite {
                sql.push_str(" LIMIT -1");
            }
            sql.push_str(&format!(" OFFSET {offset}"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawQuery {
    statement: Statement,
}

impl RawQuery {
    pub fn new(sql: impl Into<String>) -> Self {
        Self {
            statement: Statement::new(sql),
        }
    }
    pub fn bind(mut self, value: impl Into<Value>) -> Self {
        self.statement = self.statement.bind(value);
        self
    }
    pub fn statement(&self) -> &Statement {
        &self.statement
    }
    pub fn get(&self, connection: &mut dyn Connection) -> Result<Vec<Row>> {
        connection.query(&self.statement)
    }
    pub fn execute(&self, connection: &mut dyn Connection) -> Result<Execution> {
        connection.execute(&self.statement)
    }
}

fn statement(sql: String, values: Vec<Value>) -> Statement {
    values
        .into_iter()
        .fold(Statement::new(sql), |statement, value| {
            statement.bind(value)
        })
}
fn query_error(message: impl Into<String>) -> DatabaseError {
    DatabaseError::new(ErrorKind::Query, message)
}

fn unique_columns(values: &[(String, Value)]) -> Result<()> {
    for (index, (column, _)) in values.iter().enumerate() {
        if values[..index]
            .iter()
            .any(|(existing, _)| existing == column)
        {
            return Err(query_error(format!("duplicate mutation column: {column}")));
        }
    }
    Ok(())
}
