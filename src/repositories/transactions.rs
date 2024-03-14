use futures::lock::Mutex;
use mockall::automock;
use std::sync::Arc;

use crate::models::transactions::Transaction;
use crate::models::TransactionID;

pub type StoredTX = Arc<Mutex<Transaction>>;

/// The repository abstraction for the transaction storage layer.
///
/// In this case, this is meant to handle local concurrency, but it basically
/// only supports in memory storage.
/// At the moment, the only way I can think of to correctly support offsite repositories
/// is to make all modifications run by this repository, which would mean we must have
/// all of the transaction functions "mirrored" here
#[automock]
pub trait TTransactionRepository: Send + Sync {
    /// Find a tx by a given ID
    async fn find_tx_by_id(&self, tx_id: TransactionID) -> Option<StoredTX>;

    /// Indicate to the repository that we should save the changes done to the stored transaction
    /// This could be done with the Unit Of Work pattern or something similar.
    async fn save_tx(&self, tx: StoredTX);

    /// Store a tx in the repository
    ///
    /// Store a transaction that is not in the repository into the repository
    async fn store_tx(&self, tx: Transaction) -> StoredTX;
}
