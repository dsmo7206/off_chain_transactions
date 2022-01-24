# Off Chain Transactions

### Assumptions/Simplifications

I had to make a lot of assumptions when coding this as the desired behaviour isn't specified in the doc very specifically. There are comments alongside most assumptions, but I assumed:

- Frozen/locked accounts allow deposits, disputes, resolutions, and chargebacks, but not withdrawals.
- A transaction may be (disputed, resolved) infinitely many times, but once charged back, cannot be disputed again.
- The client's balance affected during a dispute is the one on the transaction referenced by the transaction_id on the dispute. The client_id mentioned directly on the dispute instruction is not used, and is not validated.
- Only deposits and withdrawals may be disputed.
- Certain errors not described in the doc are "fatal" and will halt the program, e.g. two cacheable transactions (deposits or withdrawals) having the same transaction id.

### Optimisations

- The CSV file isn't kept in memory, but streamed one record at a time.
- Given we need to store transactions in memory, I'm not storing strings.
- Memory usage could be further optimised by doing an initial pass over the CSV file to build a set of the to-be-disputed transaction ids, and then only caching those during the second pass over the file. Right now, I'm only caching deposits and withdrawals (but I'm caching _all_ of them) because those are the only disputable types.

### Warts

- The `ClientId` and `TransactionId` might be considered overkill because they're wrapping different types anyway and hence not easy to mix up.
- The `CsvFileReader` was an attempt to hide the two-phase parsing of a record (`csv::StringRecord` -> `TransactionFields` -> `Transaction`) into a single iterable but the extra code doesn't really add anything.
- It would have been nice to factor out the shared code in `State::process`'s dispute/resolve/chargeback arms.
