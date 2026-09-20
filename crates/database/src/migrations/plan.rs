use super::{
    compile_alter, compile_comments, compile_create, compile_indexes, compile_rebuild,
    compile_table_operation, AlterTable, CreateTable, RebuildTable, TableOperation,
};
use crate::{Driver, Result, Statement};

#[derive(Clone, Debug, PartialEq)]
pub enum MigrationOperation {
    Create(CreateTable),
    Alter(AlterTable),
    Table(TableOperation),
    Rebuild(RebuildTable),
    Statement(Statement),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MigrationPlan {
    operations: Vec<MigrationOperation>,
}

impl MigrationPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(mut self, table: CreateTable) -> Self {
        self.operations.push(MigrationOperation::Create(table));
        self
    }

    pub fn alter(mut self, table: AlterTable) -> Self {
        self.operations.push(MigrationOperation::Alter(table));
        self
    }

    pub fn table(mut self, operation: TableOperation) -> Self {
        self.operations.push(MigrationOperation::Table(operation));
        self
    }

    pub fn rebuild(mut self, table: RebuildTable) -> Self {
        self.operations.push(MigrationOperation::Rebuild(table));
        self
    }

    pub fn statement(mut self, statement: Statement) -> Self {
        self.operations
            .push(MigrationOperation::Statement(statement));
        self
    }

    pub fn compile(&self, driver: Driver) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();
        for operation in &self.operations {
            match operation {
                MigrationOperation::Create(table) => {
                    statements.push(compile_create(table, driver)?);
                    statements.extend(compile_indexes(table, driver)?);
                    statements.extend(compile_comments(table, driver)?);
                }
                MigrationOperation::Alter(table) => {
                    statements.extend(compile_alter(table, driver)?);
                }
                MigrationOperation::Table(operation) => {
                    statements.push(compile_table_operation(operation, driver)?);
                }
                MigrationOperation::Rebuild(table) => {
                    statements.extend(compile_rebuild(table, driver)?);
                }
                MigrationOperation::Statement(statement) => statements.push(statement.clone()),
            }
        }
        Ok(statements)
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}
