use nova_sql::NovaSql;

#[tokio::main]
async fn main() {
    // Simple in-memory sqlite example that demonstrates connecting and
    // obtaining a read/write pool. This example is intentionally minimal and
    // does not run migrations.
    let sql = NovaSql::connect("sqlite::memory:", false).await;
    let pool = sql.read_write_pool();

    // Use the pool for a read or write operation (demo only – no real query).
    let _write = pool.write();
    let _read = pool.read().await;

    println!("connected to database; replicas={}", sql.replica_count().await);
}
