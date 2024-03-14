use std::path::PathBuf;
use std::sync::Arc;
use futures::stream::BoxStream;
use futures::StreamExt;
use crate::infrastructure::in_mem_dbs::{ClientInMemRepository, TransactionInMemRepository};
use crate::models::{ClientID, TransactionID};
use crate::models::client::Client;
use crate::models::transactions::Transaction;
use crate::repositories::clients::{StoredClient, TClientRepository};
use crate::repositories::transactions::{StoredTX, TTransactionRepository};
use crate::services::transaction_service::{TransactionService, TTransactionService};
use crate::state_exporter::IStateExporter;
use crate::tx_reception::{CSVTransactionProvider, TTransactionStreamProvider};

mod models;
mod repositories;
mod services;
mod infrastructure;
mod tx_reception;
mod state_exporter;

pub(crate) const FLOATING_POINT_ACC: i32 = 4;

fn initialize_client_repo() -> impl TClientRepository {
    ClientInMemRepository::default()
}

fn initialize_transaction_repo() -> impl TTransactionRepository {
    TransactionInMemRepository::default()
}

fn initialize_service(client_repo: impl TClientRepository, transaction_repo: impl TTransactionRepository) -> impl TTransactionService {
    TransactionService::new(client_repo, transaction_repo)
}

fn initialize_tx_receiver() -> impl TTransactionStreamProvider {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 1 {
        panic!("No arguments provided");
    }

    let csv_file = args.get(0).expect("No file provided");

    let path = PathBuf::from(csv_file);

    CSVTransactionProvider::from(path)
}

fn initialize_state_exporter() -> impl IStateExporter {
    state_exporter::StateExporter
}

#[tokio::main]
async fn main() {
    let tx_receiver = initialize_tx_receiver();

    let client_repo = ShareableClientRepository::from(initialize_client_repo());
    let transaction_repo = initialize_transaction_repo();

    let transaction_service = initialize_service(client_repo.clone(), transaction_repo);

    tx_receiver.subscribe_to_tx_stream().await.for_each(|tx| async {
        if let Err(err) = transaction_service.process_transaction(tx).await {
            eprintln!("Error processing transaction: {:?}", err);
        }
    }).await;

    let state_exporter = initialize_state_exporter();

    let state = client_repo.find_all_clients().await;

    state_exporter.export_state(state).await.expect("Failed to export state");
}

pub struct ShareableTransactionRepository<TR> {
    repo: Arc<TR>,
}

pub struct ShareableClientRepository<CR> {
    repo: Arc<CR>,
}

impl<TR> From<TR> for ShareableTransactionRepository<TR> {
    fn from(repo: TR) -> Self {
        Self {
            repo: Arc::new(repo),
        }
    }
}

impl<TR> Clone for ShareableTransactionRepository<TR> {
    fn clone(&self) -> Self {
        Self {
            repo: self.repo.clone(),
        }
    }
}

impl<TR> TTransactionRepository for ShareableTransactionRepository<TR> where TR: TTransactionRepository {
    async fn find_tx_by_id(&self, tx_id: TransactionID) -> Option<StoredTX> {
        self.repo.find_tx_by_id(tx_id).await
    }

    async fn save_tx(&self, tx: StoredTX) {
        self.repo.save_tx(tx).await
    }

    async fn store_tx(&self, tx: Transaction) -> StoredTX {
        self.repo.store_tx(tx).await
    }
}

impl<CR> From<CR> for ShareableClientRepository<CR> {
    fn from(repo: CR) -> Self {
        Self {
            repo: Arc::new(repo),
        }
    }
}

impl<CR> Clone for ShareableClientRepository<CR> {
    fn clone(&self) -> Self {
        Self {
            repo: self.repo.clone(),
        }
    }
}

impl<CR> TClientRepository for ShareableClientRepository<CR> where CR: TClientRepository {
    async fn find_all_clients(&self) -> BoxStream<'static, StoredClient> {
        self.repo.find_all_clients().await
    }

    async fn find_client_by_id(&self, client_id: ClientID) -> Option<StoredClient> {
        self.repo.find_client_by_id(client_id).await
    }

    async fn save_client(&self, client: StoredClient) {
        self.repo.save_client(client).await
    }

    async fn store_client(&self, client: Client) -> StoredClient {
        self.repo.store_client(client).await
    }
}
