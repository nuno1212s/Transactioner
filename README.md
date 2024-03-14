# Assumptions made

Disputes on withdrawals do not remove money from available, instead they just add the amount to the disputed amount. This is because the money has already been withdrawn and taking it again from the available amount would be double counting.

Disputes on deposits allow the available value of the user to go into the negatives (in the case some of the money had already been withdrawn).

## Patterns used:
Utilized Domain Driven Design for the models and separation of components.

Test Driven Design to verify the correctness of certain invariants that must be maintained both at the model level and at the service level.

# Correctness

All described cases are handled by the system.

Utilized the type-state builder pattern to ensure that models can never be created while missing required fields at the type (compile) level.

Wrote unit tests to verify invariants on each of the various models and service. We also utilize the enum system to ensure that we can never have invalid states (like amounts in disputes, etc.).

## Data Store
Used a simple in-memory data store to keep track of the accounts and transactions (while using the repository pattern to allow for further changes to the data store).

Each transaction and client is wrapped in an Arc and Mutex to allow for concurrent execution of the service.

We leave an opening for a possible Unit of Work pattern in order to allow for the possibility of a more complex data store (like a database) to be used in the future.

## Efficiency

### Handling incoming transactions

Our transaction processing service receives a single transaction at a time and processes it in a thread safe way.
Then, to handle the incoming transactions we use generic streams, such that they can come from any type of producer (like a file, a network connection, etc.).
This means we can share the service across multiple threads and process multiple transaction streams concurrently.

### Reading from CSV

To read from the CSV file, we launch a blocking task (as it might be long running and we don't want to block the regular task worker pool) and then we use channels to propagate the transactions as we parse them (no entire dataset loading is done.)

# Safety and Error Handling

This was a big part of the design effort. We wanted to make sure that the service was robust and could handle any type of problem that came its way.
We use absolutely no panics in the core services, only panicking in the CSV transaction parser (as those are unrecoverable). Instead, we have a very robust and descriptive error handling system, using Rusts Results which makes for a clean, safe execution. (To make error generation easier we utilized [thiserror](https://crates.io/crates/thiserror)).

Also, to handle float precision errors, we transform all numbers into integers (by multiplying by 10^Precision) and then perform all operations on the integers. This allows us to avoid float precision errors.