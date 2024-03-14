use std::error::Error;

use thiserror::Error;

use crate::models::{ClientID, TransactionID};
use crate::models::client::{Client, ClientOperationError};
use crate::models::transactions::{Transaction, TransactionError, TransactionType};
use crate::repositories::clients::{StoredClient, TClientRepository};
use crate::repositories::transactions::TTransactionRepository;

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
                let mut client_guard = tx_client.lock().await;

                client_guard.deposit(amount.clone())?;

                // We only want to directly store the transactions which are
                // Entities in their own right.
                self.transaction_repository.store_tx(transaction).await;

                Ok(())
            }
            TransactionType::Withdrawal { amount, .. } => {
                let mut client_guard = tx_client.lock().await;

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
                        let mut tx_guard = disputed_tx.lock().await;

                        tx_guard.dispute(transaction)?;

                        let mut client_guard = tx_client.lock().await;

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
                        let mut tx_guard = disputed_tx.lock().await;

                        tx_guard.settle_dispute(transaction.clone())?;

                        let mut tx_client = tx_client.lock().await;

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
    pub(crate) fn new(client_repo: CR, transaction_repo: TR) -> Self {
        Self {
            client_repository: client_repo,
            transaction_repository: transaction_repo,
        }
    }

    /// Initialize the empty client
    async fn initialize_empty_client(&self, client_id: ClientID) -> StoredClient {
        let client = Client::builder().with_client_id(client_id).build();

        self.client_repository.store_client(client).await
    }
}

/// The processing errors for the transaction service
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
    use std::sync::{Arc};
    use futures::lock::Mutex;

    use mockall::predicate::eq;

    use crate::models::client::Client;
    use crate::models::transactions::{Transaction, TransactionType};
    use crate::repositories::clients::MockTClientRepository;
    use crate::repositories::transactions::MockTTransactionRepository;
    use crate::services::transaction_service::{TransactionProcessingError, TransactionService, TTransactionService};

    #[tokio::test]
    async fn test_deposit_transaction_processing() -> Result<(), TransactionProcessingError> {
        let mut cli_repo = MockTClientRepository::new();
        let mut tx_repo = MockTTransactionRepository::new();

        let client = {
            let client = Arc::new(Mutex::new(
                Client::builder().with_client_id(1).build()));

            cli_repo.expect_find_client_by_id()
                .with(eq(1))
                .return_const(Some(client.clone()));

            cli_repo.expect_save_client().once().return_const(());

            tx_repo.expect_store_tx()
                .times(1)
                .returning(|tx| Arc::new(Mutex::new(tx)));

            client
        };

        let tx_service = TransactionService::new(cli_repo, tx_repo);

        let test_tx = Transaction::builder()
            .with_client_id(1)
            .with_tx_type(TransactionType::Deposit {
                amount: 1000,
                dispute: None,
            })
            .with_tx_id(1)
            .build();

        tx_service.process_transaction(test_tx).await?;

        let client_guard = client.lock().await;

        assert_eq!(client_guard.available(), 1000);
        assert_eq!(client_guard.held(), 0);

        Ok(())
    }
}