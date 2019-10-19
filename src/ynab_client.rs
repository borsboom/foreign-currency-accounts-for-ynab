use chrono::NaiveDate;
use std::fmt;
use ynab_api::apis::client::APIClient;
use ynab_api::apis::configuration::{ApiKey, Configuration};
use ynab_api::models;

use crate::errors::*;
use crate::utilities::*;

pub struct YnabBudgetClient<'a> {
    client: APIClient,
    pub budget_id: &'a str,
}

// 'ynab_api::apis::Error' doesn't implement fmt::Display which makes it
// incompatible with error_chain, so we wrap it.
#[derive(Debug)]
struct YnabApiError(ynab_api::apis::Error);

impl<'a> YnabBudgetClient<'a> {
    pub fn new(api_key: String, budget_id: &'a str) -> YnabBudgetClient {
        let mut configuration = Configuration::new();
        configuration.api_key = Some(ApiKey {
            prefix: Some("Bearer".to_string()),
            key: api_key,
        });
        YnabBudgetClient {
            client: APIClient::new(configuration),
            budget_id,
        }
    }

    pub fn get_budget_settings(&self) -> Result<models::BudgetSettings> {
        Ok(self
            .client
            .budgets_api()
            .get_budget_settings_by_id(self.budget_id)
            .map_err(YnabApiError)
            .chain_err(|| "Failed to load budget settings from YNAB")?
            .data
            .settings)
    }

    pub fn get_accounts(&self) -> Result<Vec<models::Account>> {
        self.client
            .accounts_api()
            .get_accounts(self.budget_id, None)
            .map_err(YnabApiError)
            .chain_err(|| "Failed to load accounts from YNAB")
            .map(|result| result.data.accounts)
    }

    pub fn get_transactions(
        &self,
        start_date: Option<NaiveDate>,
        server_knowledge: Option<i64>,
    ) -> Result<models::TransactionsResponseData> {
        Ok(self
            .client
            .transactions_api()
            .get_transactions(
                self.budget_id,
                start_date.map(format_iso_date),
                None,
                server_knowledge,
            )
            .map_err(YnabApiError)
            .chain_err(|| "Failed to load latest transactions from YNAB")?
            .data)
    }

    pub fn create_transactions(
        &self,
        transactions: Vec<models::SaveTransaction>,
    ) -> Result<Vec<models::TransactionDetail>> {
        let wrapper = models::SaveTransactionsWrapper {
            transaction: None,
            transactions: Some(transactions),
        };
        Ok(self
            .client
            .transactions_api()
            .create_transaction(self.budget_id, wrapper)
            .map_err(YnabApiError)
            .chain_err(|| "Failed to save new transactions to YNAB")?
            .data
            .transactions
            .unwrap_or_else(|| vec![]))
    }

    pub fn update_transactions(
        &self,
        transactions: Vec<models::UpdateTransaction>,
    ) -> Result<Vec<models::TransactionDetail>> {
        let wrapper = models::UpdateTransactionsWrapper { transactions };
        Ok(self
            .client
            .transactions_api()
            .update_transactions(self.budget_id, wrapper)
            .map_err(YnabApiError)
            .chain_err(|| "Failed to save changed transactions to YNAB")?
            .data
            .transactions
            .unwrap_or_else(|| vec![]))
    }
}

impl fmt::Display for YnabApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ynab_api::apis::Error::Io(_) => write!(f, "YNAB API I/O error"),
            ynab_api::apis::Error::Reqwest(_) => write!(f, "YNAB API request error"),
            ynab_api::apis::Error::Serde(_) => write!(f, "YNAB API parse error"),
        }
    }
}

impl std::error::Error for YnabApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            ynab_api::apis::Error::Io(err) => Some(err),
            ynab_api::apis::Error::Reqwest(err) => Some(err),
            ynab_api::apis::Error::Serde(err) => Some(err),
        }
    }
}
