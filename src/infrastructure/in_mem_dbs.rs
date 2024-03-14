use std::collections::HashMap;
use std::sync::Arc;
use futures::lock::Mutex;
use futures::{stream, StreamExt};
use futures::stream::BoxStream;

use crate::models::{ClientID, TransactionID};
use crate::models::client::Client;
use crate::models::transactions::Transaction;
use crate::repositories::clients::{StoredClient, TClientRepository};
use crate::repositories::transactions::{StoredTX, TTransactionRepository};

/// The in memory repository that will
/// handle the storage of all our clients
#[derive(Default)]
pub struct ClientInMemRepository {
    stored_clients: Mutex<HashMap<ClientID, StoredClient>>,
}

/// The in memory repository
/// that will handle the storage
/// of the transaction
#[derive(Default)]
pub struct TransactionInMemRepository {
    stored_transactions: Mutex<HashMap<TransactionID, StoredTX>>,
}

impl TTransactionRepository for TransactionInMemRepository {
    async fn find_tx_by_id(&self, tx_id: TransactionID) -> Option<StoredTX> {
        let guard = self.stored_transactions.lock().await;

        guard.get(&tx_id).cloned()
    }

    async fn save_tx(&self, _tx: StoredTX) {
        // Atm, since this is only in memory, we don't actually
        // perform any changes.
    }

    async fn store_tx(&self, tx: Transaction) -> StoredTX {

        let tx_id = tx.transaction_id();

        let stored_tx = Arc::new(Mutex::new(tx));

        {
            let mut tx_guard = self.stored_transactions.lock().await;

            tx_guard.insert(tx_id, stored_tx.clone());
        }

        stored_tx
    }
}

impl TClientRepository for ClientInMemRepository {
    async fn find_all_clients(&self) -> BoxStream<'static, StoredClient> {
        let client_guard = self.stored_clients.lock().await;

        let stored_clients = client_guard.values().cloned().collect::<Vec<StoredClient>>();

        stream::iter(stored_clients).boxed()
    }

    async fn find_client_by_id(&self, client_id: ClientID) -> Option<StoredClient> {
        let client_guard = self.stored_clients.lock().await;

        client_guard.get(&client_id).cloned()
    }

    async fn save_client(&self, _client: StoredClient) {
        // Atm, since this is only in memory, we don't actually need
        // To save anything to the repository
    }

    async fn store_client(&self, client: Client) -> StoredClient {

        let cli_id = client.client_id();

        let stored_client = Arc::new(Mutex::new(client));

        {
            let mut client_guard = self.stored_clients.lock().await;

            client_guard.insert(cli_id, stored_client.clone());
        }

        stored_client
    }
}