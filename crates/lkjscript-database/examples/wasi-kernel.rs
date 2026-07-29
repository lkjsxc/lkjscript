use std::error::Error;
use std::sync::Arc;

use lkjscript_database::{Database, DatabaseLimits, Key, TenantId, Value};
use lkjscript_host::{DurableStorage, FakeDurableStorage};

fn main() -> Result<(), Box<dyn Error>> {
    let fake = Arc::new(FakeDurableStorage::new());
    let storage: Arc<dyn DurableStorage> = fake;
    let database = Database::create(
        Arc::clone(&storage),
        "wasi-kernel",
        DatabaseLimits::default(),
    )?;
    let tenant = TenantId::new(b"application-a".to_vec())?;
    let key = Key::new(b"ordered-key".to_vec())?;
    let value = Value::new(b"durable-value".to_vec())?;
    let mut write = database.begin_write()?;
    write.put(tenant.clone(), key.clone(), value.clone())?;
    write.commit()?;
    database.close()?;

    let reopened = Database::open(storage, "wasi-kernel", DatabaseLimits::default())?;
    let read = reopened.begin_read()?;
    if read.get(&tenant, &key) != Some(value) {
        return Err("replayed value mismatch".into());
    }
    println!("wasi transactional kernel replayed one committed value");
    Ok(())
}
