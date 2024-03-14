use crate::models::client::Client;
use crate::models::ClientID;
use futures::lock::Mutex;
use futures::stream::BoxStream;
use mockall::automock;
use std::sync::Arc;

pub type StoredClient = Arc<Mutex<Client>>;

/// The client repository trait, meant to represent the storage of the client
/// models.
#[automock]
pub trait TClientRepository: Send + Sync {
    /// Find all of the clients stored in this repository
    async fn find_all_clients(&self) -> BoxStream<'static, StoredClient>;

    async fn find_client_by_id(&self, client_id: ClientID) -> Option<StoredClient>;

    /// Save the changes made in this stored client instance
    ///
    /// In order to implement this in a given repository, we should use the Unit Of Work
    /// pattern.
    async fn save_client(&self, client: StoredClient);

    /// Register a client that does not yet exist in the repository
    async fn store_client(&self, client: Client) -> StoredClient;
}
