use crate::{Connection, Result};

/// An explicitly ordered, repeatable data population task.
pub trait Seeder {
    fn name(&self) -> &'static str;
    fn run(&self, connection: &mut dyn Connection) -> Result<()>;
}

pub fn run_seeders(
    connection: &mut dyn Connection,
    seeders: &[&dyn Seeder],
) -> Result<Vec<&'static str>> {
    let mut completed = Vec::with_capacity(seeders.len());
    for seeder in seeders {
        seeder.run(connection)?;
        completed.push(seeder.name());
    }
    Ok(completed)
}

/// Builds deterministic test records and keeps persistence application-defined.
pub trait Factory {
    type Output;

    fn make(&mut self, index: u64) -> Self::Output;
    fn persist(&mut self, connection: &mut dyn Connection, value: &Self::Output) -> Result<()>;

    fn make_many(&mut self, count: u64) -> Vec<Self::Output> {
        (0..count).map(|index| self.make(index)).collect()
    }

    fn create_many(
        &mut self,
        connection: &mut dyn Connection,
        count: u64,
    ) -> Result<Vec<Self::Output>> {
        let mut values = Vec::new();
        for index in 0..count {
            let value = self.make(index);
            self.persist(connection, &value)?;
            values.push(value);
        }
        Ok(values)
    }
}
