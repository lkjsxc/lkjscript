impl TenantDatabaseProvider {
    fn finish(&self, transaction: DatabaseTransactionId, commit: bool) -> HostResult<()> {
        let slot = self.validate(transaction)?;
        let transaction = self
            .lock()?
            .active
            .remove(&slot)
            .ok_or_else(unknown_transaction)?;
        match (transaction, commit) {
            (ActiveTransaction::Read(transaction), true) => {
                transaction.commit();
                Ok(())
            }
            (ActiveTransaction::Read(transaction), false) => {
                transaction.abort();
                Ok(())
            }
            (ActiveTransaction::Write(transaction), true) => {
                transaction.commit().map_err(host_error)
            }
            (ActiveTransaction::Write(transaction), false) => {
                transaction.abort().map_err(host_error)
            }
        }
    }
}

impl Drop for TenantDatabaseProvider {
    fn drop(&mut self) {
        let _ = <Self as DatabaseProvider>::abort_all(self);
    }
}

fn unknown_transaction() -> HostError {
    HostError::NotFound("database transaction".into())
}

fn wrong_transaction_kind() -> HostError {
    HostError::PermissionDenied("database transaction is read-only".into())
}
