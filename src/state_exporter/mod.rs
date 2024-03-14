use std::error::Error;

use futures::{Stream, StreamExt};
use thiserror::Error;

use crate::FLOATING_POINT_ACC;
use crate::models::client::ClientAccountStatus;
use crate::repositories::clients::StoredClient;

/// The state exporter, meant for the last part of the assignment,
/// where we have to print out the state of the clients after all
/// the transactions have been processed.
pub trait IStateExporter {
    type Error: Error + Send + Sync;

    async fn export_state(&self, state: impl Stream<Item=StoredClient>) -> Result<(), Self::Error>;
}

pub struct StateExporter;

impl IStateExporter for StateExporter {
    type Error = StateExporterError;

    async fn export_state(&self, state: impl Stream<Item=StoredClient>) -> Result<(), StateExporterError> {
        println!("client, available, held, total, locked");

        state.for_each(|client| async move {
            let client_guard = client.lock().await;

            let formatted_available = (client_guard.available() as f64) / 10.0f64.powi(FLOATING_POINT_ACC);
            let formatted_held = (client_guard.held() as f64) / 10.0f64.powi(FLOATING_POINT_ACC);
            let formatted_total = (client_guard.total() as f64) / 10.0f64.powi(FLOATING_POINT_ACC);

            let locked = match client_guard.account_status() {
                ClientAccountStatus::Active => false,
                ClientAccountStatus::Frozen => true
            };

            println!("{}, {}, {}, {}, {}", client_guard.client_id(), formatted_available, formatted_held, formatted_total, locked);
        }).await;

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum StateExporterError {
    // We don't really have any errors here, but we might as well
    // have this here for future use.
}