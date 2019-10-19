use chrono::NaiveDate;
use regex::Regex;

use crate::errors::*;

const ISO_DATE_FORMAT: &str = "%Y-%m-%d";

pub fn format_iso_date(date: NaiveDate) -> String {
    date.format(ISO_DATE_FORMAT).to_string()
}

pub fn parse_iso_date(iso_date: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(iso_date, ISO_DATE_FORMAT)
        .chain_err(|| format!("Invalid ISO date string (YYYY-MM-DD): {}", iso_date))
}

pub fn account_matches_regex<'a>(regex: &Regex, account: &'a ynab_api::models::Account) -> bool {
    if regex.is_match(&account.name) {
        true
    } else if let Some(note) = &account.note {
        regex.is_match(note)
    } else {
        false
    }
}

pub fn transaction_detail_cleared_to_save_transaction(
    cleared: ynab_api::models::transaction_detail::Cleared,
) -> ynab_api::models::save_transaction::Cleared {
    use ynab_api::models::*;
    match cleared {
        transaction_detail::Cleared::Cleared => save_transaction::Cleared::Cleared,
        transaction_detail::Cleared::Reconciled => save_transaction::Cleared::Reconciled,
        transaction_detail::Cleared::Uncleared => save_transaction::Cleared::Uncleared,
    }
}

pub fn transaction_detail_flag_color_to_save_transaction(
    flag_color: ynab_api::models::transaction_detail::FlagColor,
) -> ynab_api::models::save_transaction::FlagColor {
    use ynab_api::models::*;
    match flag_color {
        transaction_detail::FlagColor::Blue => save_transaction::FlagColor::Blue,
        transaction_detail::FlagColor::Green => save_transaction::FlagColor::Green,
        transaction_detail::FlagColor::Orange => save_transaction::FlagColor::Orange,
        transaction_detail::FlagColor::Purple => save_transaction::FlagColor::Purple,
        transaction_detail::FlagColor::Red => save_transaction::FlagColor::Red,
        transaction_detail::FlagColor::Yellow => save_transaction::FlagColor::Yellow,
    }
}

pub fn transaction_detail_cleared_to_update_transaction(
    cleared: ynab_api::models::transaction_detail::Cleared,
) -> ynab_api::models::update_transaction::Cleared {
    use ynab_api::models::*;
    match cleared {
        transaction_detail::Cleared::Cleared => update_transaction::Cleared::Cleared,
        transaction_detail::Cleared::Reconciled => update_transaction::Cleared::Reconciled,
        transaction_detail::Cleared::Uncleared => update_transaction::Cleared::Uncleared,
    }
}

pub fn transaction_detail_flag_color_to_update_transaction(
    flag_color: ynab_api::models::transaction_detail::FlagColor,
) -> ynab_api::models::update_transaction::FlagColor {
    use ynab_api::models::*;
    match flag_color {
        transaction_detail::FlagColor::Blue => update_transaction::FlagColor::Blue,
        transaction_detail::FlagColor::Green => update_transaction::FlagColor::Green,
        transaction_detail::FlagColor::Orange => update_transaction::FlagColor::Orange,
        transaction_detail::FlagColor::Purple => update_transaction::FlagColor::Purple,
        transaction_detail::FlagColor::Red => update_transaction::FlagColor::Red,
        transaction_detail::FlagColor::Yellow => update_transaction::FlagColor::Yellow,
    }
}
