use chrono::NaiveDate;
use log::debug;
use std::collections::{HashMap, HashSet};

use crate::errors::*;
use crate::types::*;
use crate::utilities::*;

#[derive(Debug)]
pub struct CurrencyConverterClient<'a> {
    api_key: &'a str,
}

impl<'a> CurrencyConverterClient<'a> {
    pub fn new(api_key: &'a str) -> CurrencyConverterClient {
        CurrencyConverterClient { api_key }
    }

    pub fn get_date_exchange_rates(
        &self,
        date: NaiveDate,
        from_to_currency_pairs: &HashSet<(CurrencyCode, CurrencyCode)>,
    ) -> Result<HashMap<(CurrencyCode, CurrencyCode), ExchangeRate>> {
        let date_iso = format_iso_date(date);
        let currency_api_url = format!(
            "https://free.currconv.com/api/v7/convert?q={}&compact=ultra&date={}&apiKey={}",
            from_to_currency_pairs
                .iter()
                .map(|(from, to)| format!("{}_{}", from, to))
                .collect::<Vec<_>>()
                .join(","),
            date_iso,
            self.api_key
        );
        println!("  Getting exchange rates from API for {}...", date);
        debug!(
            "Currency converter API historical exchange rates URL: {}",
            currency_api_url
        );
        let mut response =
            reqwest::get(&currency_api_url).chain_err(|| "Failed to get response")?;
        if response.status().is_client_error() {
            Err(Error::from(
                response
                    .json::<HashMap<String, serde_json::Value>>()
                    .chain_err(|| "Failed to parse response")?
                    .get("error")
                    .chain_err(|| format!("{} response missing error field", response.status()))?
                    .to_string(),
            ))
        } else {
            response
                .error_for_status()
                .chain_err(|| "Error response")?
                .json::<HashMap<String, HashMap<String, f64>>>()
                .chain_err(|| "Failed to parse response")?
                .into_iter()
                .map(|(code_pair, rate_map)| {
                    let &rate = rate_map
                        .get(&date_iso)
                        .chain_err(|| "Requested date missing from response")?;
                    let from = CurrencyCode::from_str(&code_pair[0..3])
                        .chain_err(|| "Invalid \"from\" currency in response response")?;
                    let to = CurrencyCode::from_str(&code_pair[4..7])
                        .chain_err(|| "Invalid \"to\" currency in response response")?;
                    Ok(((from, to), ExchangeRate::from_f64(rate)))
                })
                .collect::<Result<_>>()
        }
    }
}
