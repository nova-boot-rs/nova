use crate::connection::NovaSql;
use sea_orm::DatabaseConnection;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::RwLock;

/// Simple read/write pool with round-robin replica selection for reads.
#[derive(Clone)]
pub struct ReadWritePool {
    primary: DatabaseConnection,
    replicas: Arc<RwLock<Vec<DatabaseConnection>>>,
    rr: Arc<AtomicUsize>,
}

impl ReadWritePool {
    pub fn new(
        primary: DatabaseConnection,
        replicas: Arc<RwLock<Vec<DatabaseConnection>>>,
    ) -> Self {
        Self {
            primary,
            replicas,
            rr: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Choose a replica connection for read queries. Async because replicas list is protected by an async lock.
    pub async fn read(&self) -> DatabaseConnection {
        let reps = self.replicas.read().await;
        if reps.is_empty() {
            return self.primary.clone();
        }

        let idx = self.rr.fetch_add(1, Ordering::Relaxed);
        reps[idx % reps.len()].clone()
    }

    /// Return the primary connection for writes.
    pub fn write(&self) -> DatabaseConnection {
        self.primary.clone()
    }

    /// Add a replica connection dynamically.
    pub async fn add_replica(&self, conn: DatabaseConnection) {
        self.replicas.write().await.push(conn);
    }
}

impl NovaSql {
    /// Construct a `ReadWritePool` for injection into handlers; clones internal references.
    pub fn read_write_pool(&self) -> ReadWritePool {
        ReadWritePool::new(self.db.clone(), self.replicas.clone())
    }
}
