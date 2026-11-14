pub mod account;
pub mod ledger;

use crate::account::Account;

pub fn seed() -> Account {
    Account { balance: 0 }
}
