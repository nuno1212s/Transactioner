use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use futures::stream::BoxStream;
use futures::StreamExt;
use crate::FLOATING_POINT_ACC;

use crate::models::{ClientID, MoneyType, TransactionID};
use crate::models::transactions::{Transaction, TransactionType};


/// Transaction stream provider.
/// This should return a stream with all transactions that we want to process.
///
///TODO: Should we support various providers, or a given provider being allowed
/// to return multiple streams?
pub trait TTransactionStreamProvider {
    /// Subscribe to a transaction stream.
    ///
    /// I would have used an impl Stream<Item = Transaction> here, but that's still not
    /// stable, so we return a dynamic caller which shouldn't really loose too much performance.
    ///
    /// This consumes the entire provider as we are only meant to have a single stream.
    /// In the future, we could look at having multiple streams.
    async fn subscribe_to_tx_stream(self) -> BoxStream<'static, Transaction>;
}

pub struct CSVTransactionProvider<R> {
    file: R,
}


impl<R> TTransactionStreamProvider for CSVTransactionProvider<R>
    where R: Read + Send + 'static {
    async fn subscribe_to_tx_stream(self) -> BoxStream<'static, Transaction> {
        let (tx_sender, rx) = flume::unbounded();

        // Launch a blocking task responsible for reading the CSV file.
        // This will read from the file and send the transactions through a flume
        // Channel, which will be used to create a stream.
        tokio::task::spawn_blocking(move || {

            // Construct the csv reader from the file reader
            let mut csv_reader = csv::ReaderBuilder::new()
                .has_headers(true)
                .trim(csv::Trim::All)
                .from_reader(self.file);

            for record in csv_reader.records() {
                let csv_record = record.unwrap();

                let type_str = csv_record.get(0).unwrap();

                let client_id: ClientID = csv_record.get(1).unwrap().parse().unwrap();

                let tx_id: TransactionID = csv_record.get(2).unwrap().parse().unwrap();

                let amount_float: f64 = csv_record.get(3).unwrap().parse().unwrap();

                // Get the 4 decimal digit precision in a single integer, so we
                // Get no funny business with the floating point arithmetic.
                let amount = (amount_float * (10.0f64.powi(FLOATING_POINT_ACC))) as MoneyType;

                let tx_type = match type_str {
                    "deposit" => {
                        TransactionType::Deposit {
                            amount,
                            dispute: None,
                        }
                    }
                    "withdrawal" => {
                        TransactionType::Withdrawal {
                            amount,
                            dispute: None,
                        }
                    }
                    "dispute" => {
                        TransactionType::Dispute
                    }
                    "resolve" => {
                        TransactionType::Resolve
                    }
                    "chargeback" => {
                        TransactionType::Chargeback
                    }
                    _ => unreachable!("Transaction type is not valid")
                };

                let tx = Transaction::builder()
                    .with_client_id(client_id)
                    .with_tx_id(tx_id)
                    .with_tx_type(tx_type)
                    .build();

                tx_sender.send(tx).unwrap()
            }
        });

        rx.into_stream().boxed()
    }
}


impl From<PathBuf> for CSVTransactionProvider<File> {
    fn from(file: PathBuf) -> Self {
        CSVTransactionProvider {
            file: File::open(file).unwrap(),
        }
    }
}

#[cfg(test)]
mod reader_test {
    use std::io::BufReader;
    use crate::tx_reception::CSVTransactionProvider;
    use crate::tx_reception::TTransactionStreamProvider;
    use futures::StreamExt;
    use crate::models::transactions::TransactionType;

    #[tokio::test]
    async fn test_csv_reader() {
        const CSV_DATA: &str = "type, client, tx, amount\ndeposit, 1, 1, 1.0";

        let csv_provider = CSVTransactionProvider {
            file: BufReader::new(CSV_DATA.as_bytes())
        };

        let mut stream = csv_provider.subscribe_to_tx_stream().await;

        let tx = stream.next().await.expect("No transaction found?");

        assert_eq!(tx.client(), 1);
        assert_eq!(tx.transaction_id(), 1);

        match tx.tx_type() {
            TransactionType::Deposit { amount, dispute, .. } => {
                assert!(dispute.is_none());
                assert_eq!(*amount, 1000);
            }
            _ => panic!("Transaction type is not deposit")
        }
    }
}