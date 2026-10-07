//! Receipt accounting survives a failed unreturned-result destructor.
use super::*;

impl<R> JoinedChild<R> {
    pub(super) fn discard(&mut self) {
        let mut disposal = ResultDisposal {
            child: self,
            completion: None,
        };
        let result = disposal.child.receiver.recv();
        let reusable = result.is_ok();
        // User result destruction happens before eligibility, outside every
        // scheduler lock. The guard also returns receipt accounting on unwind.
        drop(result);
        disposal.completion = Some(reusable);
    }
}

struct ResultDisposal<'a, R> {
    child: &'a mut JoinedChild<R>,
    // None means disposal did not complete. Some(false) is a missing receipt.
    completion: Option<bool>,
}

impl<R> Drop for ResultDisposal<'_, R> {
    fn drop(&mut self) {
        if self.completion.is_none() {
            let mut state = lock(&self.child.inner.state);
            state.shutdown_error.get_or_insert_with(|| {
                ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "normalized_parallel_worker",
                    "structured child result cleanup did not complete",
                )
            });
        }
        // A failed destructor never donates its worker or becomes successful
        // cleanup. Its current owner still stops and joins the physical thread.
        self.child.release(self.completion.unwrap_or(false));
    }
}
