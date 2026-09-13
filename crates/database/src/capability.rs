#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Driver {
    Postgres,
    MySql,
    Sqlite,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Capability {
    Returning,
    Savepoints,
    TransactionalDdl,
    AdvisoryLocks,
    ReadOnlyTransactions,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Capabilities(u32);

impl Capabilities {
    pub const fn new() -> Self {
        Self(0)
    }
    pub const fn with(self, capability: Capability) -> Self {
        Self(self.0 | bit(capability))
    }
    pub const fn supports(self, capability: Capability) -> bool {
        self.0 & bit(capability) != 0
    }
}

const fn bit(capability: Capability) -> u32 {
    match capability {
        Capability::Returning => 1 << 0,
        Capability::Savepoints => 1 << 1,
        Capability::TransactionalDdl => 1 << 2,
        Capability::AdvisoryLocks => 1 << 3,
        Capability::ReadOnlyTransactions => 1 << 4,
    }
}
