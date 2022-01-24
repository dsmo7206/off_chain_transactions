use super::FixedFloat;
use std::{convert::TryFrom, error::Error};

// A "type-safe" transaction id. Probably overkill!
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransactionId(pub u32);

impl std::fmt::Display for TransactionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// A "type-safe" client id. Probably overkill!
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u16);

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone)]
pub struct Transaction {
    pub transaction_id: TransactionId,
    pub client_id: ClientId,
    pub inner: TransactionInner,
    pub state: TransactionState,
}

impl Transaction {
    pub fn new(
        transaction_id: TransactionId,
        client_id: ClientId,
        inner: TransactionInner,
    ) -> Self {
        Self {
            transaction_id,
            client_id,
            inner,
            state: TransactionState::Alive,
        }
    }
}

impl TryFrom<TransactionFields> for Transaction {
    type Error = TransactionFieldsError;

    fn try_from(fields: TransactionFields) -> Result<Self, Self::Error> {
        Ok(Transaction::new(
            TransactionId(fields.transaction_id),
            ClientId(fields.client_id),
            match fields.type_.as_str() {
                "deposit" => TransactionInner::Deposit(
                    fields
                        .amount
                        .ok_or(TransactionFieldsError::DepositMissingAmount)?
                        .into(),
                ),
                "withdrawal" => TransactionInner::Withdrawal(
                    fields
                        .amount
                        .ok_or(TransactionFieldsError::WithdrawalMissingAmount)?
                        .into(),
                ),
                "dispute" => TransactionInner::Dispute,
                "resolve" => TransactionInner::Resolve,
                "chargeback" => TransactionInner::Chargeback,
                other => return Err(TransactionFieldsError::UnrecognisedType(other.into())),
            },
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionState {
    Alive,
    Disputed,
    ChargedBack,
}

#[derive(Clone)]
pub enum TransactionInner {
    Deposit(FixedFloat),
    Withdrawal(FixedFloat),
    Dispute,
    Resolve,
    Chargeback,
}

/// An intermediate type to leverage the serde deserialisation provided by the csv crate.
/// We save a bit of memory by not storing these in the `State`, but instead storing the slimmer
/// `Transaction` type. It should be possible to avoid this intermediate type by overloading
/// various `serde` functions, but it would probably be quite fiddly.
#[derive(serde::Deserialize, Debug)]
pub struct TransactionFields {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "client")]
    pub client_id: u16,
    #[serde(rename = "tx")]
    pub transaction_id: u32,
    pub amount: Option<f64>,
}

/// This error is returned when the fields of the transaction as parsed don't make sense.
#[derive(Debug)]
pub enum TransactionFieldsError {
    DepositMissingAmount,
    WithdrawalMissingAmount,
    UnrecognisedType(String),
}

impl std::fmt::Display for TransactionFieldsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DepositMissingAmount => write!(f, "Deposit \"amount\" field is blank"),
            Self::WithdrawalMissingAmount => write!(f, "Withdrawal \"amount\" field is blank"),
            Self::UnrecognisedType(other) => {
                write!(f, "Unrecognised transaction type \"{}\"", other)
            }
        }
    }
}

impl Error for TransactionFieldsError {}
