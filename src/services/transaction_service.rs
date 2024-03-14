use std::error::Error;
use thiserror::Error;
use crate::models::client::{Client, ClientOperationError};
use crate::models::{ClientID, TransactionID};
use crate::models::transactions::{Transaction, TransactionError, TransactionType};
use crate::repositories::clients::{StoredClient, TClientRepository};
use crate::repositories::transactions::{StoredTX, TTransactionRepository};

/// The transaction processing service.
/// Meant to process individual transactions taking into account a state of the system.
pub trait TTransactionService: Send + Sync {
    type Error: Error + Send + Sync;

    /// Process a given transaction.
    async fn process_transaction(&self, transaction: Transaction) -> Result<(), Self::Error>;
}

/// The transaction service, meant to handle transactions
pub struct TransactionService<CR, TR> {
    client_repository: CR,
    transaction_repository: TR,
}

impl<CR, TR> TTransactionService for TransactionService<CR, TR>
    where CR: TClientRepository,
          TR: TTransactionRepository {
    type Error = TransactionProcessingError;

    async fn process_transaction(&self, transaction: Transaction) -> Result<(), Self::Error> {
        let tx_client = match self.client_repository.find_client_by_id(transaction.client()).await {
            None => {
                self.initialize_empty_client(transaction.client()).await
            }
            Some(client) => { client }
        };

        let tx_processing_result = match transaction.tx_type() {
            TransactionType::Deposit { amount, .. } => {
                let mut client_guard = tx_client.lock().unwrap();

                client_guard.deposit(amount.clone())?;

                // We only want to directly store the transactions which are
                // Entities in their own right.
                self.transaction_repository.store_tx(transaction).await;

                Ok(())
            }
            TransactionType::Withdrawal { amount, .. } => {
                let mut client_guard = tx_client.lock().unwrap();

                client_guard.withdraw(amount.clone())?;

                // We only want to directly store the transactions which are
                // Entities in their own right.
                self.transaction_repository.store_tx(transaction).await;

                Ok(())
            }
            TransactionType::Dispute => {
                match self.transaction_repository.find_tx_by_id(transaction.transaction_id()).await {
                    None => {
                        return Err(TransactionProcessingError::DisputedTransactionDoesNotExist(transaction.transaction_id()));
                    }
                    Some(disputed_tx) => {
                        let mut tx_guard = disputed_tx.lock().unwrap();

                        tx_guard.dispute(transaction)?;

                        let mut client_guard = tx_client.lock().unwrap();

                        match tx_guard.tx_type() {
                            TransactionType::Deposit { amount, .. } => {
                                client_guard.dispute_deposited_funds(amount.clone())?;
                            }
                            TransactionType::Withdrawal { amount, .. } => {
                                client_guard.dispute_withdrawn_funds(amount.clone())?;
                            }
                            _ => unreachable!("Transaction type is not valid")
                        }
                    }
                };

                Ok(())
            }
            TransactionType::Resolve | TransactionType::Chargeback => {
                match self.transaction_repository.find_tx_by_id(transaction.transaction_id()).await {
                    None => {
                        return Err(TransactionProcessingError::SettledDisputedTransactionDoesNotExist(transaction.transaction_id()));
                    }
                    Some(disputed_tx) => {
                        let mut tx_guard = disputed_tx.lock().unwrap();

                        tx_guard.settle_dispute(transaction)?;

                        let mut tx_client = tx_client.lock().unwrap();

                        match transaction.tx_type() {
                            TransactionType::Resolve => {
                                tx_client.resolve_funds(tx_guard.amount()?)?;
                            }
                            TransactionType::Chargeback => {
                                tx_client.chargeback_funds(tx_guard.amount()?)?;
                            }
                            _ => {
                                // This is unreachable as we have just checked it in the previous match
                                unreachable!()
                            }
                        }
                    }
                };

                Ok(())
            }
        };

        self.client_repository.save_client(tx_client).await;

        tx_processing_result
    }
}

impl<CR, TR> TransactionService<CR, TR> where CR: TClientRepository {
    async fn initialize_empty_client(&self, client_id: ClientID) -> StoredClient {
        let client = Client::builder().with_client_id(client_id).build();

        self.client_repository.store_client(client).await
    }
}

#[derive(Error, Debug)]
pub enum TransactionProcessingError {
    #[error("Client error {0:?}")]
    ClientError(#[from] ClientOperationError),
    #[error("Transaction error {0:?}")]
    TransactionError(#[from] TransactionError),
    #[error("The disputed transaction does not exist")]
    DisputedTransactionDoesNotExist(TransactionID),
    #[error("The settled dispute transaction does not exist")]
    SettledDisputedTransactionDoesNotExist(TransactionID),
}

#[cfg(test)]
mod service_tests {

}