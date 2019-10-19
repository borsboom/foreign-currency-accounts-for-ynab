use chrono::NaiveDate;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::currency_converter_client::*;
use crate::database::*;
use crate::errors::*;
use crate::types::*;

pub struct ExchangeRatesCache<'a> {
    currency_converter_client: &'a CurrencyConverterClient<'a>,
    database: &'a Database,
    cache: RefCell<HashMap<(CurrencyCode, NaiveDate), ExchangeRate>>,
}

impl<'a> ExchangeRatesCache<'a> {
    pub fn new(
        currency_converter_client: &'a CurrencyConverterClient<'a>,
        database: &'a Database,
    ) -> ExchangeRatesCache<'a> {
        ExchangeRatesCache {
            currency_converter_client,
            database,
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_exchange_rate(
        &self,
        anticipate_from_currencies: &'a HashSet<CurrencyCode>,
        from_currency: CurrencyCode,
        to_currency: CurrencyCode,
        date: NaiveDate,
    ) -> Result<ExchangeRate> {
        let cache_key = (from_currency, date);
        let mut cache = self.cache.borrow_mut();
        if let Some(&rate) = cache.get(&cache_key) {
            return Ok(rate);
        }
        let rate_result = self
            .database
            .get_exchange_rate(from_currency, to_currency, date)?;
        if let Some(rate) = rate_result {
            cache.insert(cache_key, rate);
            return Ok(rate);
        }
        let mut loaded_rates = self.database.get_known_exchange_rates(
            anticipate_from_currencies,
            to_currency,
            date,
        )?;
        let currencies_to_get_from_api: HashSet<(CurrencyCode, CurrencyCode)> =
            anticipate_from_currencies
                .difference(&loaded_rates.keys().cloned().collect())
                .map(|&code| (code, to_currency))
                .collect();
        let currency_converter_response = self
            .currency_converter_client
            .get_date_exchange_rates(date, &currencies_to_get_from_api)
            .chain_err(|| "Failed to get exchange rate from Currency Converter API")?;
        for (got_currency, _) in currencies_to_get_from_api {
            let &got_exchange_rate = currency_converter_response
                .get(&(got_currency, to_currency))
                .chain_err(|| {
                    format!(
                        "Response is missing exchange rate for currency: {}",
                        got_currency
                    )
                })?;
            self.database
                .put_exchange_rate(got_currency, to_currency, date, got_exchange_rate)?;
            loaded_rates.insert(got_currency, got_exchange_rate);
        }
        let &rate = loaded_rates.get(&from_currency).chain_err(|| {
            format!(
                "Response is missing exchange rate for currency: {}",
                from_currency
            )
        })?;
        cache.insert(cache_key, rate);
        Ok(rate)
    }
}
