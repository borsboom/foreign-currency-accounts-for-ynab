use chrono::NaiveDate;
use std::collections::HashSet;

use crate::types::*;

#[derive(Debug)]
pub struct BudgetState {
    pub start_date: NaiveDate,
    pub ynab_server_knowledge: Option<i64>,
    pub last_run_date: Option<NaiveDate>,
}

#[derive(Debug)]
pub struct UpdateBudgetState<'a> {
    pub had_changes: bool,
    pub create_difference_transactions: Vec<CreateDifferenceTransaction<'a>>,
    pub update_difference_transactions: Vec<DifferenceTransaction<'a>>,
    pub delete_difference_transaction_ids: HashSet<YnabTransactionId<'a>>,
}

#[derive(Debug)]
pub struct CreateDifferenceTransaction<'a> {
    pub foreign_transaction_id: YnabTransactionId<'a>,
    pub inner: DifferenceTransaction<'a>,
}

#[derive(Debug)]
pub struct DifferenceTransaction<'a> {
    pub difference_transaction_id: YnabTransactionId<'a>,
    pub amount: Milliunits,
    pub difference_key: DifferenceKey,
    pub transfer_key: Option<DifferenceKey>,
}
