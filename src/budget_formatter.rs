use chrono::NaiveDate;
use rust_decimal::prelude::Zero;
use rust_decimal::{Decimal, RoundingStrategy};
use std::cell::RefCell;
use std::str;

use crate::types::*;

#[derive(Debug)]
pub struct BudgetFormatter<'a> {
    settings: &'a ynab_api::models::BudgetSettings,
    date_format: RefCell<Option<String>>,
}

impl<'a> BudgetFormatter<'a> {
    pub fn new(settings: &ynab_api::models::BudgetSettings) -> BudgetFormatter {
        BudgetFormatter {
            settings,
            date_format: RefCell::new(None),
        }
    }

    pub fn format_milliunits_with_code(
        &self,
        currency: CurrencyCode,
        amount: Milliunits,
    ) -> String {
        self.format_milliunits_custom(&currency.to_str(), " ", true, false, amount)
    }

    pub fn format_milliunits(&self, amount: Milliunits) -> String {
        self.format_milliunits_custom(
            &self.settings.currency_format.currency_symbol,
            "",
            false,
            true,
            amount,
        )
    }

    pub fn format_exchange_rate(&self, exchange_rate: ExchangeRate) -> String {
        self.format_currency_custom(
            &self.settings.currency_format.currency_symbol,
            "",
            false,
            true,
            true,
            exchange_rate.to_decimal(),
        )
    }

    pub fn format_date(&self, date: NaiveDate) -> String {
        let mut fmt_opt = self.date_format.borrow_mut();
        let fmt = fmt_opt.get_or_insert_with(|| {
            self.settings
                .date_format
                .format
                .replace("YYYY", "%Y")
                .replace("MM", "%m")
                .replace("DD", "%d")
        });
        date.format(fmt).to_string()
    }

    fn format_milliunits_custom(
        &self,
        currency_symbol: &str,
        currency_symbol_spacer: &str,
        force_display_symbol: bool,
        minus_before_symbol_first: bool,
        amount: Milliunits,
    ) -> String {
        self.format_currency_custom(
            currency_symbol,
            currency_symbol_spacer,
            force_display_symbol,
            minus_before_symbol_first,
            false,
            amount.to_decimal(),
        )
    }

    fn format_currency_custom(
        &self,
        currency_symbol: &str,
        currency_symbol_spacer: &str,
        force_display_symbol: bool,
        minus_before_symbol_first: bool,
        all_decimal_digits: bool,
        amount: Decimal,
    ) -> String {
        let currency_format = &self.settings.currency_format;
        let abs_amount: Decimal = amount.abs();
        let raw_formatted = if all_decimal_digits {
            format!("{}", abs_amount)
        } else {
            format!(
                "{:.*}",
                currency_format.decimal_digits as usize,
                abs_amount.round_dp_with_strategy(
                    currency_format.decimal_digits as u32,
                    RoundingStrategy::RoundHalfUp
                )
            )
        };
        let split_around_decimal: Vec<&str> = raw_formatted.split('.').collect();
        let group_separated_before_decimal = self.add_group_separators(
            split_around_decimal
                .get(0)
                .expect("split_around_decimal should have two elements"),
        );
        let group_separated = format!(
            "{}{}{}{}",
            if !minus_before_symbol_first && amount < Decimal::zero() {
                "-"
            } else {
                ""
            },
            group_separated_before_decimal,
            currency_format.decimal_separator,
            split_around_decimal
                .get(1)
                .expect("split_around_decimal should have two elements")
        );
        let group_separated_with_symbol = if currency_format.display_symbol || force_display_symbol
        {
            if currency_format.symbol_first {
                format!(
                    "{}{}{}",
                    currency_symbol, currency_symbol_spacer, group_separated
                )
            } else {
                format!(
                    "{}{}{}",
                    group_separated, currency_symbol_spacer, currency_symbol
                )
            }
        } else {
            group_separated
        };
        if minus_before_symbol_first && amount < Decimal::zero() {
            format!("-{}", group_separated_with_symbol)
        } else {
            group_separated_with_symbol
        }
    }

    fn add_group_separators(&self, before_decimal: &str) -> String {
        before_decimal
            .chars()
            .rev()
            .collect::<Vec<char>>()
            .chunks(3)
            .map(|chunk| chunk.iter().collect())
            .collect::<Vec<String>>()
            .join(&self.settings.currency_format.group_separator)
            .chars()
            .rev()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;

    lazy_static! {
        pub static ref US_SETTINGS: ynab_api::models::BudgetSettings =
            ynab_api::models::BudgetSettings {
                date_format: ynab_api::models::DateFormat {
                    format: "MM/DD/YYYY".to_string(),
                },
                currency_format: ynab_api::models::CurrencyFormat {
                    iso_code: "USD".to_string(),
                    example_format: "$1.23".to_string(),
                    decimal_digits: 2,
                    decimal_separator: ".".to_string(),
                    symbol_first: true,
                    group_separator: ",".to_string(),
                    currency_symbol: "$".to_string(),
                    display_symbol: true,
                }
            };
        pub static ref OTHER_SETTINGS: ynab_api::models::BudgetSettings =
            ynab_api::models::BudgetSettings {
                date_format: ynab_api::models::DateFormat {
                    format: "YYYY-MM-DD".to_string(),
                },
                currency_format: ynab_api::models::CurrencyFormat {
                    iso_code: "ABC".to_string(),
                    example_format: "$1.23".to_string(),
                    decimal_digits: 3,
                    decimal_separator: ",".to_string(),
                    symbol_first: false,
                    group_separator: ".".to_string(),
                    currency_symbol: "X".to_string(),
                    display_symbol: true,
                }
            };
        pub static ref NO_SYMBOL_SETTINGS: ynab_api::models::BudgetSettings =
            ynab_api::models::BudgetSettings {
                date_format: ynab_api::models::DateFormat {
                    format: "YYYY-MM-DD".to_string(),
                },
                currency_format: ynab_api::models::CurrencyFormat {
                    iso_code: "ABC".to_string(),
                    example_format: "$1.23".to_string(),
                    decimal_digits: 3,
                    decimal_separator: ",".to_string(),
                    symbol_first: false,
                    group_separator: ".".to_string(),
                    currency_symbol: "X".to_string(),
                    display_symbol: false,
                }
            };
    }

    #[test]
    fn test_format_milliunits() {
        assert_eq!(
            BudgetFormatter::new(&US_SETTINGS)
                .format_milliunits(Milliunits::from_scaled_i64(-12_345)),
            "-$12.35"
        );
        assert_eq!(
            BudgetFormatter::new(&OTHER_SETTINGS)
                .format_milliunits(Milliunits::from_scaled_i64(-12_345)),
            "-12,345X"
        );
        assert_eq!(
            BudgetFormatter::new(&NO_SYMBOL_SETTINGS)
                .format_milliunits(Milliunits::from_scaled_i64(-123_456)),
            "-123,456"
        );
        assert_eq!(
            BudgetFormatter::new(&US_SETTINGS)
                .format_milliunits(Milliunits::from_scaled_i64(123_456_789_012_345)),
            "$123,456,789,012.35"
        );
        assert_eq!(
            BudgetFormatter::new(&US_SETTINGS).format_milliunits(Milliunits::from_scaled_i64(123)),
            "$0.12"
        );
    }

    #[test]
    fn test_format_milliunits_with_code() {
        assert_eq!(
            BudgetFormatter::new(&US_SETTINGS).format_milliunits_with_code(
                CurrencyCode::from_str("USD").unwrap(),
                Milliunits::from_scaled_i64(-12_345_678)
            ),
            "USD -12,345.68"
        );
        assert_eq!(
            BudgetFormatter::new(&OTHER_SETTINGS).format_milliunits_with_code(
                CurrencyCode::from_str("ABC").unwrap(),
                Milliunits::from_scaled_i64(-12_345_678)
            ),
            "-12.345,678 ABC"
        );
        assert_eq!(
            BudgetFormatter::new(&NO_SYMBOL_SETTINGS).format_milliunits_with_code(
                CurrencyCode::from_str("XXY").unwrap(),
                Milliunits::from_scaled_i64(-123_456_789)
            ),
            "-123.456,789 XXY"
        );
    }

    #[test]
    fn test_format_date() {
        assert_eq!(
            BudgetFormatter::new(&US_SETTINGS).format_date(NaiveDate::from_ymd(2011, 4, 27)),
            "04/27/2011"
        );
        assert_eq!(
            BudgetFormatter::new(&OTHER_SETTINGS).format_date(NaiveDate::from_ymd(2011, 4, 27)),
            "2011-04-27"
        );
    }
}
