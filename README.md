# Assumptions made

Disputes on withdrawals do not remove money from available, instead they just add the amount to the disputed amount. This is because the money has already been withdrawn and taking it again from the available amount would be double counting.

Disputes on deposits allow the available value of the user to go into the negatives (in the case some of the money had already been withdrawn).

## Patterns used:
Utilized Domain Driven Design for the models and separation of components.
Test Driven Design to verify the correctness of certain invariants that must be maintained both at the model level and at the service level.

Wrote unit tests to verify invariants on each of the various components.
Used a simple in-memory data store to keep track of the accounts and transactions (while using the repository pattern to allow for further changes to the data store).