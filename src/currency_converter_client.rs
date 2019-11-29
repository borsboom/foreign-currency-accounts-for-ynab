use chrono::NaiveDate;
use log::debug;
use std::collections::{HashMap, HashSet};

use crate::errors::*;
use crate::types::*;
use crate::utilities::*;

#[derive(Debug)]
pub struct CurrencyConverterClient<'a> {
    api_key: &'a str,
    base_url: &'a str,
    max_pairs_per_request: usize,
}

impl<'a> CurrencyConverterClient<'a> {
    pub fn new(api_key: &'a str, base_url: &'a str, max_pairs_per_request: usize) -> Self {
        CurrencyConverterClient {
            api_key,
            base_url,
            max_pairs_per_request,
        }
    }

    pub fn get_date_exchange_rates(
        &self,
        date: NaiveDate,
        from_to_currency_pairs: &HashSet<(CurrencyCode, CurrencyCode)>,
    ) -> Result<HashMap<(CurrencyCode, CurrencyCode), ExchangeRate>> {
        let date_iso = format_iso_date(date);
        println!("  Getting exchange rates from API for {}...", date);
        let mut results = HashMap::new();
        for currency_pairs_chunk in from_to_currency_pairs
            .iter()
            .cloned()
            .collect::<Vec<(CurrencyCode, CurrencyCode)>>()
            .chunks(self.max_pairs_per_request)
        {
            let currency_api_url = format!(
                "{}/api/v7/convert?q={}&compact=ultra&date={}&apiKey={}",
                self.base_url,
                currency_pairs_chunk
                    .iter()
                    .map(|(from, to)| format!("{}_{}", from, to))
                    .collect::<Vec<_>>()
                    .join(","),
                date_iso,
                self.api_key
            );
            debug!(
                "Currency converter API historical exchange rates URL: {}",
                currency_api_url
            );
            let mut response =
                reqwest::get(&currency_api_url).chain_err(|| "Failed to get response")?;
            if response.status().is_client_error() {
                return Err(Error::from(
                    response
                        .json::<HashMap<String, serde_json::Value>>()
                        .chain_err(|| "Failed to parse response")?
                        .get("error")
                        .chain_err(|| {
                            format!("{} response missing error field", response.status())
                        })?
                        .to_string(),
                ));
            } else {
                let results_chunk: HashMap<(CurrencyCode, CurrencyCode), ExchangeRate> = response
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
                    .collect::<Result<_>>()?;
                results.extend(results_chunk);
            }
        }
        Ok(results)
    }
}
