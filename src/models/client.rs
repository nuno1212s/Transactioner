use getset::{CopyGetters, Getters};
use thiserror::Error;
use crate::models::{ClientID, MoneyType, NoVal};

/// The current status of the account
#[derive(PartialEq, Eq, Default)]
pub enum ClientAccountStatus {
    #[default]
    Active,
    Frozen,
}

#[derive(Getters, CopyGetters)]
pub struct Client {
    #[get_copy = "pub"]
    client_id: ClientID,
    #[get_copy = "pub"]
    available: MoneyType,
    #[get_copy = "pub"]
    held: MoneyType,
    #[get]
    account_status: ClientAccountStatus,
}

impl Client {
    pub fn builder() -> ClientBuilder<NoVal> {
        Default::default()
    }

    pub fn total(&self) -> MoneyType {
        self.available + self.held
    }

    pub fn deposit(&mut self, amount: MoneyType) -> Result<(), ClientOperationError> {
        if let ClientAccountStatus::Frozen = self.account_status {
            return Err(ClientOperationError::AccountFrozen);
        }

        self.available += amount;

        Ok(())
    }

    pub fn withdraw(&mut self, amount: MoneyType) -> Result<(), ClientOperationError> {
        if let ClientAccountStatus::Frozen = self.account_status {
            return Err(ClientOperationError::AccountFrozen);
        }

        if amount >= self.available {
            return Err(WithdrawFundsError::NotEnoughFunds(self.available, amount).into());
        }

        self.available -= amount;

        Ok(())
    }

    /// When we are disputing a deposit transaction, we must remove the available funds
    /// and move them to the held category
    pub fn dispute_deposited_funds(&mut self, amount: MoneyType) -> Result<(), ClientOperationError> {
        if let ClientAccountStatus::Frozen = self.account_status {
            return Err(ClientOperationError::AccountFrozen);
        }

        // When disputing deposited funds, we allow the available funds to go negative
        self.available -= amount;
        self.held += amount;

        Ok(())
    }

    /// When disputing withdrawn funds, we do not remove the available funds from the account
    /// Since that would lead to "double" spending
    pub fn dispute_withdrawn_funds(&mut self, amount: MoneyType) -> Result<(), ClientOperationError> {
        if let ClientAccountStatus::Frozen = self.account_status {
            return Err(ClientOperationError::AccountFrozen);
        }

        self.held += amount;

        Ok(())
    }

    /// Charge back a given amount of funds, this will move the funds from the held
    pub fn chargeback_funds(&mut self, amount: MoneyType) -> Result<(), ClientOperationError> {
        if let ClientAccountStatus::Frozen = self.account_status {
            return Err(ClientOperationError::AccountFrozen);
        }

        if self.held < amount {
            return Err(ChargeBackError::NotEnoughHeldFunds(self.held, amount).into());
        }

        self.held -= amount;
        self.account_status = ClientAccountStatus::Frozen;

        Ok(())
    }

    pub fn resolve_funds(&mut self, amount: MoneyType) -> Result<(), ClientOperationError> {
        if let ClientAccountStatus::Frozen = self.account_status {
            return Err(ClientOperationError::AccountFrozen);
        }

        if self.held < amount {
            return Err(ResolveError::NotEnoughHeldFunds(self.held, amount).into());
        }

        self.held -= amount;
        self.available += amount;

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum DepositFundsError {}

#[derive(Error, Debug)]
pub enum WithdrawFundsError {
    #[error("The account does not have enough funds ({0:?} while trying to withdraw {1:?})")]
    NotEnoughFunds(MoneyType, MoneyType)
}

#[derive(Error, Debug)]
pub enum DisputeFundsError {
    #[error("Failed to dispute transaction, not enough funds available to dispute")]
    DisputedFundsNotAvailable(MoneyType, MoneyType)
}

#[derive(Error, Debug)]
pub enum ChargeBackError {
    #[error("Attempting to charge back a larger amount than what is held. Held value: {0:?} charging back {1:?}")]
    NotEnoughHeldFunds(MoneyType, MoneyType)
}

#[derive(Error, Debug)]
pub enum ResolveError {
    #[error("Attempting to resolve funds that are larger than the amount of funds that we are holding. Held value {0:?}, resolving {1:?}")]
    NotEnoughHeldFunds(MoneyType, MoneyType)
}

/// A wrapper for all client errors, so they can be more easily propagated
/// upwards, without actually knowing all of the individual ones
#[derive(Error, Debug)]
pub enum ClientOperationError {
    #[error("Cannot deposit funds as the account is frozen")]
    AccountFrozen,
    #[error("Deposit Error {0:?}")]
    DepositError(#[from] DepositFundsError),
    #[error("Withdraw Error {0:?}")]
    WithdrawError(#[from] WithdrawFundsError),
    #[error("Dispute Error {0:?}")]
    DisputeError(#[from] DisputeFundsError),
    #[error("Chargeback Error {0:?}")]
    ChargebackError(#[from] ChargeBackError),
    #[error("Resolve Error {0:?}")]
    ResolveError(#[from] ResolveError),
}

/// Using the type state builder pattern for compile type safety
///
/// In this case, when constructing a builder we can accept not setting the
/// available and held, as it will be assumed as 0, therefore we don't
/// need those generic types.
pub struct ClientBuilder<CLID> {
    client_id: CLID,
    available: MoneyType,
    held: MoneyType,
    account_status: ClientAccountStatus,
}

impl<CLID> ClientBuilder<CLID> {
    pub fn with_available(mut self, available: MoneyType) -> Self {
        self.available = available;

        self
    }

    pub fn with_held(mut self, held: MoneyType) -> Self {
        self.held = held;

        self
    }

    pub fn with_account_status(mut self, status: ClientAccountStatus) -> Self {
        self.account_status = status;

        self
    }
}

impl ClientBuilder<NoVal> {
    pub fn with_client_id(self, client_id: ClientID) -> ClientBuilder<ClientID> {
        ClientBuilder {
            client_id,
            available: self.available,
            held: self.held,
            account_status: self.account_status,
        }
    }
}

impl ClientBuilder<ClientID> {
    pub fn build(self) -> Client {
        Client {
            client_id: self.client_id,
            available: self.available,
            held: self.held,
            account_status: self.account_status,
        }
    }
}

impl Default for ClientBuilder<NoVal> {
    fn default() -> Self {
        ClientBuilder {
            client_id: Default::default(),
            available: Default::default(),
            held: Default::default(),
            account_status: Default::default(),
        }
    }
}

#[cfg(test)]
mod client_tests {
    use crate::models::client::{Client, ClientAccountStatus};

    #[test]
    pub fn test_client_init() {
        let client = Client::builder()
            .with_client_id(1)
            .build();
    }

    #[test]
    pub fn test_negative_withdrawal() {
        let mut client = Client::builder()
            .with_client_id(1)
            .build();

        assert!(client.withdraw(1).is_err())
    }

    #[test]
    pub fn test_frozen_movement() {
        let mut client = Client::builder()
            .with_client_id(1)
            .with_available(100)
            .with_held(100)
            .with_account_status(ClientAccountStatus::Frozen)
            .build();

        assert!(client.withdraw(1).is_err());
        assert!(client.deposit(1).is_err());
    }

    #[test]
    pub fn test_overflow_held() {
        let mut client = Client::builder()
            .with_client_id(1)
            .build();

        assert!(client.resolve_funds(100).is_err());
        assert!(client.chargeback_funds(100).is_err());
    }

    #[test]
    pub fn test_resolved_dispute() {
        let mut client = Client::builder()
            .with_client_id(1)
            .build();

        client.deposit(100).unwrap();

        client.dispute_deposited_funds(100).unwrap();

        assert_eq!(client.available(), 0);
        assert_eq!(client.held(), 100);

        client.resolve_funds(100).unwrap();

        assert_eq!(client.available(), 100);
        assert_eq!(client.held(), 0);
        assert_eq!(client.total(), 100);
    }

    #[test]
    pub fn test_charged_back_dispute() {
        let mut client = Client::builder()
            .with_client_id(1)
            .build();

        client.deposit(100).unwrap();

        client.dispute_deposited_funds(100).unwrap();

        assert_eq!(client.available(), 0);
        assert_eq!(client.held(), 100);

        client.chargeback_funds(100).unwrap();

        assert_eq!(client.available(), 0);
        assert_eq!(client.held(), 0);
        assert_eq!(client.total(), 0);
        match client.account_status() {
            ClientAccountStatus::Active => {
                panic!("Account should be frozen")
            }
            _ => {}
        }
    }
}