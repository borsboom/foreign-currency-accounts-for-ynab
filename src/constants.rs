use lazy_static::lazy_static;
use regex::Regex;

use crate::types::*;

pub const IMPORT_ID_PREFIX: &str = "FCAY";
pub const ADJUSTMENT_MEMO_PREFIX: &str = "Exchange rate adjustment: ";
pub const DIFFERENCE_MEMO_TAG_PREFIX: &str = "CONVERT: ";
pub const DEFAULT_DATABASE_FILENAME: &str = "data.sqlite3";
pub const DEFAULT_CURRENCY_CONVERTER_API_BASE_URL: &str = "https://free.currconv.com";
pub const DEFAULT_CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST: usize = 2;
pub const SAVE_TRANSACTIONS_CHUNK_SIZE: usize = 50;

pub const YES_ARG: &str = "yes";
pub const AUTO_APPROVE_TRANSACTIONS_ARG: &str = "auto-approve-transactions";
pub const AUTO_APPROVE_TRANSACTIONS_ENV: &str = "FCAY_AUTO_APPROVE_TRANSACTIONS";
pub const AUTO_APPROVE_ADJUSTMENTS_ARG: &str = "auto-approve-adjustments";
pub const AUTO_APPROVE_ADJUSTMENTS_ENV: &str = "FCAY_AUTO_APPROVE_ADJUSTMENTS";
pub const YNAB_ACCESS_TOKEN_ARG: &str = "ynab-access-token";
pub const YNAB_ACCESS_TOKEN_ENV: &str = "YNAB_ACCESS_TOKEN";
pub const CURRENCY_CONVERTER_API_KEY_ARG: &str = "currency-converter-api-key";
pub const CURRENCY_CONVERTER_API_KEY_ENV: &str = "CURRENCY_CONVERTER_API_KEY";
pub const CURRENCY_CONVERTER_API_BASE_URL_ARG: &str = "currency-converter-base-url";
pub const CURRENCY_CONVERTER_API_BASE_URL_ENV: &str = "CURRENCY_CONVERTER_API_BASE_URL";
pub const CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST_ARG: &str =
    "currency-converter-max-currency-pairs-per-request";
pub const CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST_ENV: &str =
    "CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST";
pub const YNAB_BUDGET_ID_ARG: &str = "budget-id";
pub const YNAB_BUDGET_ID_ENV: &str = "YNAB_BUDGET_ID";
pub const START_DATE_ARG: &str = "start-date";
pub const DATABASE_FILE_ARG: &str = "database-file";
pub const DATABASE_FILE_ENV: &str = "FCAY_DATABASE_FILE";
pub const POSSIBLE_BOOL_VALUES: [&str; 2] = ["true", "false"];

lazy_static! {
    pub static ref FORCE_CONVERT_REGEX: Regex =
        Regex::new(r"(?i)<CONVERT>").expect("FORCE_CONVERT_REGEX should be valid");
    pub static ref FORCE_NO_CONVERT_REGEX: Regex =
        Regex::new(r"(?i)<NO[\s-]*CONVERT>").expect("FORCE_NO_CONVERT_REGEX should be valid");
    pub static ref ACCOUNT_CURRENCY_REGEX: Regex =
        Regex::new(r"(?i)<([[:alpha:]]{3})>").expect("ACCOUNT_CURRENCY_REGEX should be valid");
    pub static ref DIFFERENCE_ACCOUNT_CURRENCY_REGEX: Regex =
        Regex::new(r"(?i)<([[:alpha:]]{3})[\s-]+DIFFERENCE>")
            .expect("DIFFERENCE_ACCOUNT_CURRENCY_REGEX should be valid");
}

pub fn format_adjustment_payee_name(key: DifferenceKey) -> String {
    format!("Exchange Rate Adjustment <{}>", key.currency,)
}
