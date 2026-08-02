impl DatabaseProvider for TenantDatabaseProvider {
    fn begin_read(&self) -> HostResult<DatabaseTransactionId> {
        let transaction = self.database.begin_read().map_err(host_error)?;
        self.insert(ActiveTransaction::Read(transaction))
    }

    fn begin_write(&self) -> HostResult<DatabaseTransactionId> {
        let transaction = self.database.begin_write().map_err(host_error)?;
        self.insert(ActiveTransaction::Write(transaction))
    }

    fn get(&self, transaction: DatabaseTransactionId, key: &[u8]) -> HostResult<Option<Vec<u8>>> {
        let slot = self.validate(transaction)?;
        let key = Key::new(key.to_vec()).map_err(host_error)?;
        let state = self.lock()?;
        let value = match state.active.get(&slot) {
            Some(ActiveTransaction::Read(transaction)) => transaction.get(&self.tenant, &key),
            Some(ActiveTransaction::Write(transaction)) => transaction.get(&self.tenant, &key),
            None => return Err(unknown_transaction()),
        };
        Ok(value.map(|value| value.as_bytes().to_vec()))
    }

    fn put(
        &self,
        transaction: DatabaseTransactionId,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> HostResult<()> {
        let slot = self.validate(transaction)?;
        let key = Key::new(key).map_err(host_error)?;
        let value = Value::new(value).map_err(host_error)?;
        let mut state = self.lock()?;
        match state.active.get_mut(&slot) {
            Some(ActiveTransaction::Write(transaction)) => transaction
                .put(self.tenant.clone(), key, value)
                .map_err(host_error),
            Some(ActiveTransaction::Read(_)) => Err(wrong_transaction_kind()),
            None => Err(unknown_transaction()),
        }
    }

    fn delete(&self, transaction: DatabaseTransactionId, key: Vec<u8>) -> HostResult<()> {
        let slot = self.validate(transaction)?;
        let key = Key::new(key).map_err(host_error)?;
        let mut state = self.lock()?;
        match state.active.get_mut(&slot) {
            Some(ActiveTransaction::Write(transaction)) => transaction
                .delete(self.tenant.clone(), key)
                .map_err(host_error),
            Some(ActiveTransaction::Read(_)) => Err(wrong_transaction_kind()),
            None => Err(unknown_transaction()),
        }
    }

    fn range(
        &self,
        transaction: DatabaseTransactionId,
        start: &[u8],
        end: &[u8],
        limit: usize,
    ) -> HostResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let slot = self.validate(transaction)?;
        let start = (!start.is_empty()).then_some(start);
        let end = (!end.is_empty()).then_some(end);
        let state = self.lock()?;
        let values = match state.active.get(&slot) {
            Some(ActiveTransaction::Read(transaction)) => transaction.bounded_range(
                &self.tenant,
                start,
                end,
                limit,
                MAX_PROVIDER_RANGE_BYTES,
            ),
            Some(ActiveTransaction::Write(transaction)) => transaction.bounded_range(
                &self.tenant,
                start,
                end,
                limit,
                MAX_PROVIDER_RANGE_BYTES,
            ),
            None => return Err(unknown_transaction()),
        }
        .map_err(host_error)?
        .ok_or_else(|| HostError::Io {
            operation: "range database transaction".into(),
            message: "provider returned byte bound reached".into(),
        })?;
        Ok(values
            .into_iter()
            .map(|(key, value)| (key.as_bytes().to_vec(), value.as_bytes().to_vec()))
            .collect())
    }

    fn commit(&self, transaction: DatabaseTransactionId) -> HostResult<()> {
        self.finish(transaction, true)
    }

    fn abort(&self, transaction: DatabaseTransactionId) -> HostResult<()> {
        self.finish(transaction, false)
    }

    fn abort_all(&self) -> HostResult<usize> {
        let active = {
            let mut state = self.lock()?;
            std::mem::take(&mut state.active)
        };
        let count = active.len();
        for transaction in active.into_values() {
            match transaction {
                ActiveTransaction::Read(transaction) => transaction.abort(),
                ActiveTransaction::Write(transaction) => {
                    transaction.abort().map_err(host_error)?;
                }
            }
        }
        Ok(count)
    }
}
