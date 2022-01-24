use super::types::{
    ClientId, FixedFloat, Transaction, TransactionId, TransactionInner, TransactionState,
};
use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
};

#[derive(Default)]
pub struct State {
    transactions: HashMap<TransactionId, Transaction>,
    accounts: HashMap<ClientId, AccountState>,
}

impl State {
    pub fn process(&mut self, txn: Transaction) -> Result<(), ProcessError> {
        match txn.inner {
            TransactionInner::Deposit(amount) => {
                let account = self.get_or_create_account(txn.client_id);

                // Assume we can deposit into a frozen account
                account.available += amount;

                self.cache_transaction(txn)?;
            }
            TransactionInner::Withdrawal(amount) => {
                let account = self.get_or_create_account(txn.client_id);

                // Assume we can't withdraw from a frozen account

                if !account.locked {
                    if account.available >= amount {
                        account.available -= amount;
                    }

                    // Only cache if the account isn't locked. If this withdrawal were to be
                    // disputed (is that even possible?), we wouldn't want to negate it, so
                    // just don't cache it, and the dispute code will think it's an "error on
                    // the partner side" - that's probably good enough.
                    self.cache_transaction(txn)?;
                }
            }
            TransactionInner::Dispute => {
                // Grab the disputed transaction. If it doesn't exist, just ignore and return
                let disputed_txn = match self.transactions.get_mut(&txn.transaction_id) {
                    Some(disputed_txn) => disputed_txn,
                    None => {
                        // Error on partner side
                        return Ok(());
                    }
                };

                if !matches!(disputed_txn.state, TransactionState::Alive) {
                    // Cannot dispute if already disputed or charged back
                    return Ok(());
                }

                // Fetch the disputed amount. The problem description implies this is for
                // deposits only, but presumably each deposit may have a corresponding
                // withdrawal. To handle that we just neg the amount
                let amount = match disputed_txn.inner {
                    TransactionInner::Deposit(amount) => amount,
                    TransactionInner::Withdrawal(amount) => -amount,
                    _ => return Err(ProcessError::DisputeTargetInvalid(txn.transaction_id)),
                };

                // Does the client_id on the disputed_txn need to match the one on the txn,
                // or is txn.client_id the client doing the disputing? Not clear. Either way,
                // we'll want to negate the amount on the disputed_txn's client.

                // Fetch the client. We know that the transactions happen in chronological order,
                // so the client should exist already.
                let account = match self.accounts.get_mut(&disputed_txn.client_id) {
                    Some(account) => account,
                    None => {
                        return Err(ProcessError::DisputedTransactionClientMissing(
                            disputed_txn.client_id,
                        ));
                    }
                };

                // Everything seems fine, so do all mutations
                disputed_txn.state = TransactionState::Disputed;
                account.available -= amount;
                account.held += amount;
            }
            TransactionInner::Resolve => {
                // Grab the disputed transaction. If it doesn't exist, just ignore and return
                let disputed_txn = match self.transactions.get_mut(&txn.transaction_id) {
                    Some(disputed_txn) => disputed_txn,
                    None => {
                        // Error on partner side
                        return Ok(());
                    }
                };

                if !matches!(disputed_txn.state, TransactionState::Disputed) {
                    // Not disputed; do nothing
                    return Ok(());
                }

                // Fetch the disputed amount. The problem description implies this is for
                // deposits only, but presumably each deposit may have a corresponding
                // withdrawal. To handle that we just neg the amount.
                let amount = match disputed_txn.inner {
                    TransactionInner::Deposit(amount) => amount,
                    TransactionInner::Withdrawal(amount) => -amount,
                    _ => return Err(ProcessError::DisputeTargetInvalid(txn.transaction_id)),
                };

                // Does the client_id on the disputed_txn need to match the one on the txn,
                // or is txn.client_id the client doing the disputing? Not clear. Either way,
                // we'll want to negate the amount on the disputed_txn's client.

                // Fetch the client. We know that the transactions happen in chronological order,
                // so the client should exist already.
                let account = match self.accounts.get_mut(&disputed_txn.client_id) {
                    Some(account) => account,
                    None => {
                        return Err(ProcessError::DisputedTransactionClientMissing(
                            disputed_txn.client_id,
                        ));
                    }
                };

                // Everything seems fine, so do all mutations
                disputed_txn.state = TransactionState::Alive;
                account.available += amount;
                account.held -= amount;
            }
            TransactionInner::Chargeback => {
                // Grab the disputed transaction. If it doesn't exist, just ignore and return
                let disputed_txn = match self.transactions.get_mut(&txn.transaction_id) {
                    Some(disputed_txn) => disputed_txn,
                    None => {
                        // Error on partner side
                        return Ok(());
                    }
                };

                if !matches!(disputed_txn.state, TransactionState::Disputed) {
                    // Not disputed; do nothing
                    return Ok(());
                }

                // Fetch the disputed amount. The problem description implies this is for
                // deposits only, but presumably each deposit may have a corresponding
                // withdrawal. To handle that we just neg the amount
                let amount = match disputed_txn.inner {
                    TransactionInner::Deposit(amount) => amount,
                    TransactionInner::Withdrawal(amount) => -amount,
                    _ => return Err(ProcessError::DisputeTargetInvalid(txn.transaction_id)),
                };

                // Does the client_id on the disputed_txn need to match the one on the txn,
                // or is txn.client_id the client doing the disputing? Not clear. Either way,
                // we'll want to negate the amount on the disputed_txn's client.

                // Fetch the client. We know that the transactions happen in chronological order,
                // so the client should exist already.
                let account = self.accounts.get_mut(&disputed_txn.client_id).ok_or(
                    ProcessError::DisputedTransactionClientMissing(disputed_txn.client_id),
                )?;

                // Everything seems fine, so do all mutations
                disputed_txn.state = TransactionState::ChargedBack;
                account.held -= amount;
                account.locked = true;
            }
        }

        Ok(())
    }

    fn cache_transaction(&mut self, txn: Transaction) -> Result<(), ProcessError> {
        match self.transactions.entry(txn.transaction_id) {
            Entry::Occupied(_) => Err(ProcessError::DuplicateTransactionId(txn.transaction_id)),
            Entry::Vacant(entry) => {
                entry.insert(txn);
                Ok(())
            }
        }
    }

    fn get_or_create_account(&mut self, client_id: ClientId) -> &mut AccountState {
        self.accounts
            .entry(client_id)
            .or_insert_with(AccountState::default)
    }

    pub fn write<Writer: std::io::Write>(self, mut f: Writer) -> Result<(), std::io::Error> {
        writeln!(f, "client,available,held,total,locked")?;

        for (client_id, account_state) in self.accounts {
            writeln!(
                f,
                "{},{},{},{},{}",
                client_id,
                account_state.available,
                account_state.held,
                account_state.available + account_state.held,
                account_state.locked
            )?;
        }

        Ok(())
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct AccountState {
    available: FixedFloat,
    held: FixedFloat,
    locked: bool,
}

#[derive(Debug)]
pub enum ProcessError {
    DisputedTransactionClientMissing(ClientId),
    DisputeTargetInvalid(TransactionId),
    DuplicateTransactionId(TransactionId),
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisputedTransactionClientMissing(client_id) => {
                write!(
                    f,
                    "Disputed transaction refers to non-existent client id: {}",
                    client_id
                )
            }
            Self::DisputeTargetInvalid(transaction_id) => {
                write!(
                    f,
                    "Dispute of non-disputable transaction id: {}",
                    transaction_id
                )
            }
            Self::DuplicateTransactionId(transaction_id) => {
                write!(f, "Duplicate transaction id: {}", transaction_id)
            }
        }
    }
}

impl Error for ProcessError {}

#[cfg(test)]
mod tests {
    use super::{ClientId, ProcessError, State, TransactionId, TransactionInner};
    use crate::{
        state::AccountState,
        types::{Transaction, TransactionState},
    };
    use std::collections::HashMap;

    fn build_state(txns: &[Transaction]) -> Result<State, ProcessError> {
        let mut state = State::default();

        for txn in txns {
            state.process(txn.clone())?;
        }

        Ok(state)
    }

    #[test]
    fn test_basic_example() {
        let state = build_state(&[
            Transaction::new(
                TransactionId(1),
                ClientId(1),
                TransactionInner::Deposit(1.0.into()),
            ),
            Transaction::new(
                TransactionId(2),
                ClientId(2),
                TransactionInner::Deposit(2.0.into()),
            ),
            Transaction::new(
                TransactionId(3),
                ClientId(1),
                TransactionInner::Deposit(2.0.into()),
            ),
            Transaction::new(
                TransactionId(4),
                ClientId(1),
                TransactionInner::Withdrawal(1.5.into()),
            ),
            Transaction::new(
                TransactionId(5),
                ClientId(2),
                TransactionInner::Withdrawal(3.0.into()),
            ),
        ])
        .unwrap();

        assert_eq!(
            state.accounts,
            HashMap::from_iter([
                (
                    ClientId(1),
                    AccountState {
                        available: 1.5.into(),
                        held: 0.0.into(),
                        locked: false
                    }
                ),
                (
                    ClientId(2),
                    AccountState {
                        available: 2.0.into(),
                        held: 0.0.into(),
                        locked: false
                    }
                )
            ],)
        );
    }

    #[test]
    fn test_failed_withdrawal() {
        let state = build_state(&[
            Transaction::new(
                TransactionId(1),
                ClientId(1),
                TransactionInner::Deposit(1.0.into()),
            ),
            Transaction::new(
                TransactionId(2),
                ClientId(1),
                TransactionInner::Withdrawal(2.0.into()),
            ),
        ])
        .unwrap();

        assert_eq!(
            state.accounts,
            HashMap::from_iter([(
                ClientId(1),
                AccountState {
                    available: 1.0.into(),
                    held: 0.0.into(),
                    locked: false
                }
            )])
        );
    }

    #[test]
    fn test_dispute_deposit() {
        let state = build_state(&[
            Transaction::new(
                TransactionId(1),
                ClientId(1),
                TransactionInner::Deposit(1.0.into()),
            ),
            Transaction::new(TransactionId(1), ClientId(1), TransactionInner::Dispute),
        ])
        .unwrap();

        assert_eq!(
            state.accounts,
            HashMap::from_iter([(
                ClientId(1),
                AccountState {
                    available: 0.0.into(),
                    held: 1.0.into(),
                    locked: false
                }
            )])
        );
    }

    #[test]
    fn test_dispute_withdrawal() {
        let state = build_state(&[
            Transaction::new(
                TransactionId(1),
                ClientId(1),
                TransactionInner::Deposit(5.0.into()),
            ),
            Transaction::new(
                TransactionId(2),
                ClientId(1),
                TransactionInner::Withdrawal(3.0.into()),
            ),
            Transaction::new(TransactionId(2), ClientId(1), TransactionInner::Dispute),
        ])
        .unwrap();

        assert_eq!(
            state.accounts,
            HashMap::from_iter([(
                ClientId(1),
                AccountState {
                    available: 5.0.into(),
                    held: (-3.0).into(),
                    locked: false
                }
            )])
        );

        assert_eq!(
            state.transactions.get(&TransactionId(2)).unwrap().state,
            TransactionState::Disputed
        );
    }

    #[test]
    fn test_resolve() {
        let mut state = build_state(&[
            Transaction::new(
                TransactionId(1),
                ClientId(1),
                TransactionInner::Deposit(1.0.into()),
            ),
            Transaction::new(TransactionId(1), ClientId(1), TransactionInner::Dispute),
            Transaction::new(TransactionId(1), ClientId(1), TransactionInner::Resolve),
        ])
        .unwrap();

        assert_eq!(
            state.accounts,
            HashMap::from_iter([(
                ClientId(1),
                AccountState {
                    available: 1.0.into(),
                    held: 0.0.into(),
                    locked: false
                }
            )])
        );

        assert_eq!(
            state.transactions.get(&TransactionId(1)).unwrap().state,
            TransactionState::Alive
        );

        // Try disputing/resolving again
        state
            .process(Transaction::new(
                TransactionId(1),
                ClientId(1),
                TransactionInner::Dispute,
            ))
            .unwrap();
        state
            .process(Transaction::new(
                TransactionId(1),
                ClientId(1),
                TransactionInner::Resolve,
            ))
            .unwrap();

        assert_eq!(
            state.accounts,
            HashMap::from_iter([(
                ClientId(1),
                AccountState {
                    available: 1.0.into(),
                    held: 0.0.into(),
                    locked: false
                }
            )])
        );

        assert_eq!(
            state.transactions.get(&TransactionId(1)).unwrap().state,
            TransactionState::Alive
        );
    }

    #[test]
    fn test_chargeback() {
        let mut state = build_state(&[
            Transaction::new(
                TransactionId(1),
                ClientId(1),
                TransactionInner::Deposit(123.0.into()),
            ),
            Transaction::new(
                TransactionId(2),
                ClientId(1),
                TransactionInner::Deposit(456.0.into()),
            ),
            Transaction::new(TransactionId(1), ClientId(1), TransactionInner::Dispute),
            Transaction::new(TransactionId(1), ClientId(1), TransactionInner::Chargeback),
        ])
        .unwrap();

        assert_eq!(
            state.accounts,
            HashMap::from_iter([(
                ClientId(1),
                AccountState {
                    available: 456.0.into(),
                    held: 0.0.into(),
                    locked: true
                }
            )])
        );

        assert_eq!(
            state.transactions.get(&TransactionId(1)).unwrap().state,
            TransactionState::ChargedBack
        );

        // Try disputing again; this shouldn't return an error, but the transaction
        // will stay in the ChargedBack state with no further changes.
        state
            .process(Transaction::new(
                TransactionId(1),
                ClientId(1),
                TransactionInner::Dispute,
            ))
            .unwrap();

        assert_eq!(
            state.accounts,
            HashMap::from_iter([(
                ClientId(1),
                AccountState {
                    available: 456.0.into(),
                    held: 0.0.into(),
                    locked: true
                }
            )])
        );

        assert_eq!(
            state.transactions.get(&TransactionId(1)).unwrap().state,
            TransactionState::ChargedBack
        );
    }
}
