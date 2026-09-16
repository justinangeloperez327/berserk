#[cfg(feature = "sqlite")]
mod sqlite {
    use framework_database::{drivers::sqlite::SqliteConnection, Database};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn cloned_database_handles_acquire_independent_connections() {
        let acquisitions = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&acquisitions);
        let database = Database::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            SqliteConnection::in_memory()
        });

        let clone = database.clone();
        let mut first = database.acquire().unwrap();
        let mut second = clone.acquire().unwrap();

        first.ping().unwrap();
        second.ping().unwrap();
        assert_eq!(acquisitions.load(Ordering::SeqCst), 2);
    }
}
