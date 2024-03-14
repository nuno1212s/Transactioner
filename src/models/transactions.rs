use getset::{CopyGetters, Getters};
use thiserror::Error;
use crate::models::{ClientID, MoneyType, NoVal, TransactionID};


/// The transaction model, representing a transaction made in the
/// system.
///
/// Contains the transaction ID and type, the client who is targeted by it
/// and the corresponding amount
#[derive(Getters, CopyGetters, Debug, Clone)]
pub struct Transaction {
    #[getset(get_copy = "pub")]
    transaction_id: TransactionID,
    #[getset(get = "pub")]
    tx_type: TransactionType,
    #[getset(get_copy = "pub")]
    client: ClientID,
}

/// The type of transaction we are attempting to perform
///
/// Since the amount is only present for deposit and withdrawal transactions,
/// we include it right here, in order to make it clear that the other ones
/// DO NOT POSSESS AMOUNTS, instead they use the client
/// This way, we can, at compile time, assert that all transactions
/// are well-formed
#[derive(Debug, Clone)]
pub enum TransactionType {
    Deposit {
        amount: MoneyType,
        dispute: Option<Box<Dispute>>,
    },
    Withdrawal {
        amount: MoneyType,
        dispute: Option<Box<Dispute>>,
    },
    Dispute,
    Resolve,
    Chargeback,
}

/// The dispute model.
/// Since dispute and resolution transactions don't have their own ID,
/// we will treat them as a sort of Value Object, which will not live on without
/// being attached to the original transaction.
/// This way we can successfully handle wrongful disputes or resolutions by just discarding
/// them and we better represent the expected behaviour in the model
#[derive(Debug, Clone, Getters)]
pub struct Dispute {
    #[get = "pub"]
    dispute_transaction: Transaction,

    resolution: Option<Transaction>,
}

impl Transaction {

    /// Function to initialize the transaction
    pub fn builder() -> TransactionBuilder<NoVal, NoVal, NoVal> {
        Default::default()
    }

    pub fn amount(&self) -> Result<MoneyType, TransactionError> {
        match self.tx_type {
            TransactionType::Deposit { amount, .. } | TransactionType::Withdrawal { amount, .. } => {

                Ok(amount.clone())

            }
            _ => Err(TransactionError::IllegalAmountCheck)
        }
    }

    /// Attempt to dispute this transaction with the given dispute_tx
    /// transaction
    pub fn dispute(&mut self, dispute_tx: Transaction) -> Result<(), TransactionError> {
        if let TransactionType::Dispute = dispute_tx.tx_type() {
            if dispute_tx.transaction_id != self.transaction_id {
                return Err(TransactionDisputeError::TransactionNotDisputingThisOne(self.transaction_id, dispute_tx.transaction_id).into());
            }

            return match &mut self.tx_type {
                TransactionType::Deposit { dispute, .. } | TransactionType::Withdrawal { dispute, .. } => {
                    if dispute.is_some() {
                        return Err(TransactionDisputeError::TransactionAlreadyDisputed.into());
                    }

                    let _ = dispute.insert(Box::new(Dispute {
                        dispute_transaction: dispute_tx,
                        resolution: None,
                    }));

                    Ok(())
                }
                _ => {
                    Err(TransactionDisputeError::TransactionNotDisputable.into())
                }
            };
        }

        Err(TransactionDisputeError::ProvidedTransactionNotDispute.into())
    }

    /// Settle the dispute ongoing in this transaction
    pub fn settle_dispute(&mut self, dispute_settlement: Transaction) -> Result<(), TransactionError> {
        match dispute_settlement.tx_type() {
            TransactionType::Resolve | TransactionType::Chargeback => {
                if dispute_settlement.transaction_id != self.transaction_id {
                    return Err(TransactionResolveDisputeError::TransactionNotResolvingThisOne(self.transaction_id, dispute_settlement.transaction_id).into());
                }

                match &mut self.tx_type {
                    TransactionType::Deposit { dispute, .. } | TransactionType::Withdrawal { dispute, .. } => {
                        if dispute.is_none() {
                            return Err(TransactionDisputeError::TransactionNotDisputable.into());
                        }

                        let dispute_ref = dispute.as_mut().unwrap();

                        if let Some(_) = dispute_ref.resolution {
                            return Err(TransactionResolveDisputeError::DisputeAlreadyResolved.into());
                        }

                        dispute_ref.resolution = Some(dispute_settlement);

                        Ok(())
                    }
                    _ => Err(TransactionDisputeError::TransactionNotDisputable.into()),
                }
            }
            _ => {
                Err(TransactionResolveDisputeError::ProvidedTransactionNotResolution.into())
            }
        }
    }
}

/// The transaction related errors that we can produce while maintaining the various
/// invariants of the model
#[derive(Error, Debug)]
pub enum TransactionDisputeError {
    #[error("This transaction cannot be disputed.")]
    TransactionNotDisputable,
    #[error("The provided transaction is not a dispute transaction.")]
    ProvidedTransactionNotDispute,
    #[error("Transaction has already been disputed")]
    TransactionAlreadyDisputed,
    #[error("The transaction is not disputing the current one (Current {0:?}, Disputed {1:?})")]
    TransactionNotDisputingThisOne(TransactionID, TransactionID),
}

#[derive(Error, Debug)]
pub enum TransactionResolveDisputeError {
    #[error("Failed to resolve due to {0:?}")]
    DisputeError(#[from] TransactionDisputeError),
    #[error("Cannot resolve a dispute in a transaction that is not disputed")]
    TransactionNotDisputed,
    #[error("The provided transaction is not a dispute resolution")]
    ProvidedTransactionNotResolution,
    #[error("The transaction is not resolving the current one (Current {0:?}, Disputed {1:?})")]
    TransactionNotResolvingThisOne(TransactionID, TransactionID),
    #[error("This dispute has already been resolved")]
    DisputeAlreadyResolved,
}

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("Dispute error {0:?}")]
    DisputeError(#[from] TransactionDisputeError),
    #[error("Resolve dispute error {0:?}")]
    ResolveDisputeError(#[from] TransactionResolveDisputeError),
    #[error("Cannot check the amount of this transaction")]
    IllegalAmountCheck
}


/// Implement the type state builder pattern,
/// allowing us to assert that no malformed transactions are ever created, at compile time.
/// We can also maintain other invariants at runtime, but the biggest advantage of this pattern
/// is it compile time safety.
pub struct TransactionBuilder<TID, TTY, CLID> {
    transaction_id: TID,
    tx_type: TTY,
    client_id: CLID,
}

impl<TTY, CLID> TransactionBuilder<NoVal, TTY, CLID> {
    pub fn with_tx_id(self, transaction_id: TransactionID) -> TransactionBuilder<TransactionID, TTY, CLID> {
        TransactionBuilder {
            transaction_id,
            tx_type: self.tx_type,
            client_id: self.client_id,
        }
    }
}

impl<TID, CLID> TransactionBuilder<TID, NoVal, CLID> {
    pub fn with_tx_type(self, tx_type: TransactionType) -> TransactionBuilder<TID, TransactionType, CLID> {
        TransactionBuilder {
            transaction_id: self.transaction_id,
            tx_type,
            client_id: self.client_id,
        }
    }
}

impl<TID, TTY> TransactionBuilder<TID, TTY, NoVal> {
    pub fn with_client_id(self, client_id: ClientID) -> TransactionBuilder<TID, TTY, ClientID> {
        TransactionBuilder {
            transaction_id: self.transaction_id,
            tx_type: self.tx_type,
            client_id,
        }
    }
}

impl TransactionBuilder<TransactionID, TransactionType, ClientID> {
    pub fn build(self) -> Transaction {
        Transaction {
            transaction_id: self.transaction_id,
            tx_type: self.tx_type,
            client: self.client_id,
        }
    }
}

impl Default for TransactionBuilder<NoVal, NoVal, NoVal> {
    fn default() -> Self {
        Self {
            transaction_id: Default::default(),
            tx_type: Default::default(),
            client_id: Default::default(),
        }
    }
}

#[cfg(test)]
mod transaction_tests {
    use crate::models::transactions::{Transaction, TransactionType};

    #[test]
    pub fn test_valid_transaction_init() {
        let transaction = Transaction::builder()
            .with_tx_id(1)
            .with_tx_type(TransactionType::Deposit {
                amount: 10000,
                dispute: None,
            })
            .with_client_id(2).build();

        assert_eq!(transaction.transaction_id(), 1);
        assert_eq!(transaction.client(), 2);
    }

    #[test]
    pub fn test_transaction_dispute() {
        let mut transaction = Transaction::builder()
            .with_tx_id(1)
            .with_tx_type(TransactionType::Deposit {
                amount: 10000,
                dispute: None,
            })
            .with_client_id(2).build();

        let dispute_tx = Transaction::builder()
            .with_tx_id(1)
            .with_tx_type(TransactionType::Dispute)
            .with_client_id(2).build();

        assert!(transaction.dispute(dispute_tx.clone()).is_ok());
        assert!(transaction.dispute(dispute_tx).is_err());

        let resolved_tx = Transaction::builder()
            .with_tx_id(1)
            .with_tx_type(TransactionType::Resolve)
            .with_client_id(2).build();

        assert!(transaction.settle_dispute(resolved_tx).is_ok());
    }

    #[test]
    pub fn test_dispute_with_wrong_tx() {
        let mut transaction = Transaction::builder()
            .with_tx_id(1)
            .with_tx_type(TransactionType::Deposit {
                amount: 10000,
                dispute: None,
            })
            .with_client_id(2).build();

        let fake_dispute = Transaction::builder()
            //WRONG ID
            .with_tx_id(2)
            .with_tx_type(TransactionType::Dispute)
            .with_client_id(2).build();

        assert!(transaction.dispute(fake_dispute.clone()).is_err());
        assert!(transaction.settle_dispute(fake_dispute).is_err());

        let fake_dispute = Transaction::builder()
            .with_tx_id(1)
            // WRONG TYPE
            .with_tx_type(TransactionType::Resolve)
            .with_client_id(2).build();

        assert!(transaction.dispute(fake_dispute.clone()).is_err());
        assert!(transaction.settle_dispute(fake_dispute).is_err());

        let fake_dispute = Transaction::builder()
            .with_tx_id(1)
            // WRONG TYPE
            .with_tx_type(TransactionType::Chargeback)
            .with_client_id(2).build();

        assert!(transaction.dispute(fake_dispute.clone()).is_err());
        assert!(transaction.settle_dispute(fake_dispute).is_err());
    }

    #[test]
    pub fn test_dispute_settlement() {
        let mut transaction = Transaction::builder()
            .with_tx_id(1)
            .with_tx_type(TransactionType::Deposit {
                amount: 10000,
                dispute: None,
            })
            .with_client_id(2).build();

        let valid_dispute = Transaction::builder()
            .with_tx_id(1)
            .with_tx_type(TransactionType::Dispute)
            .with_client_id(2).build();

        assert!(transaction.dispute(valid_dispute.clone()).is_ok());
        assert!(transaction.dispute(valid_dispute).is_err());

        let invalid_settlement = Transaction::builder()
            // WRONG ID
            .with_tx_id(2)
            .with_tx_type(TransactionType::Resolve)
            .with_client_id(2).build();

        assert!(transaction.settle_dispute(invalid_settlement).is_err());

        let valid_settlement = Transaction::builder()
            .with_tx_id(1)
            .with_tx_type(TransactionType::Resolve)
            .with_client_id(2).build();

        assert!(transaction.settle_dispute(valid_settlement).is_ok());
    }
}