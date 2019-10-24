use log::debug;
use regex::Regex;
use std::collections::{HashMap, HashSet};

use crate::budget_formatter::*;
use crate::constants::*;
use crate::errors::*;
use crate::types::*;
use crate::utilities::*;
use crate::ynab_client::*;

#[derive(Debug)]
pub struct ForeignAccounts<'a> {
    all_used_foreign_currencies: HashSet<CurrencyCode>,
    account_id_data: HashMap<YnabAccountId<'a>, AccountData>,
    difference_account_ids: HashMap<DifferenceKey, YnabAccountId<'a>>,
}

pub struct DifferenceBalances {
    balances: HashMap<DifferenceKey, ForeignTotalAndDifferenceBalance>,
}

pub struct ForeignTotalAndDifferenceBalance {
    pub foreign_accounts_total: Milliunits,
    pub difference_account_balance: Milliunits,
}

#[derive(Debug)]
pub enum AccountData {
    Local { force_convert: bool },
    Foreign { difference_key: DifferenceKey },
    Difference { difference_key: DifferenceKey },
}

impl<'a> ForeignAccounts<'a> {
    pub fn load(
        ynab_client: &YnabBudgetClient,
        budget_formatter: &BudgetFormatter,
        local_currency: CurrencyCode,
    ) -> Result<(ForeignAccounts<'a>, DifferenceBalances)> {
        let mut all_used_foreign_currencies = HashSet::new();
        let mut account_id_data = HashMap::new();
        let mut difference_account_ids = HashMap::new();
        println!("Getting accounts from YNAB...");
        let raw_accounts = ynab_client.get_accounts()?;
        debug!("Accounts received from YNAB: {:#?}", &raw_accounts);
        for account in &raw_accounts {
            if !account.deleted && !account.closed {
                let local_account_data = || AccountData::Local {
                    force_convert: account_matches_regex(&FORCE_CONVERT_REGEX, &account),
                };
                let account_id = YnabAccountId::new(account.id.clone());
                let opt_foreign_account_key = Self::foreign_account_key(&account)?;
                let opt_difference_account_key = Self::difference_account_key(&account)?;
                if opt_foreign_account_key.is_some() && opt_difference_account_key.is_some() {
                    bail!(
                        "One account may not be both foreign currency and difference account: {}",
                        account.name
                    );
                }
                let account_data = if let Some(difference_key) = opt_foreign_account_key {
                    if difference_key.currency == local_currency {
                        local_account_data()
                    } else {
                        all_used_foreign_currencies.insert(difference_key.currency);
                        println!(
                            "  Found foreign {}: {} ({})",
                            difference_key,
                            account.name,
                            budget_formatter
                                .format_milliunits(Milliunits::from_scaled_i64(account.balance))
                        );
                        AccountData::Foreign { difference_key }
                    }
                } else if let Some(difference_key) = opt_difference_account_key {
                    ensure!(
                        difference_key.currency != local_currency,
                        format!(
                            "Budget may not have a difference account for the local currency: {}",
                            account.name
                        )
                    );
                    ensure!(
                        difference_account_ids
                            .insert(difference_key, account_id.clone())
                            .is_none(),
                        format!(
                            "Budget may not have more than one difference {}",
                            difference_key
                        )
                    );
                    println!(
                        "  Found difference {}: {} ({})",
                        difference_key,
                        account.name,
                        budget_formatter
                            .format_milliunits(Milliunits::from_scaled_i64(account.balance))
                    );
                    AccountData::Difference { difference_key }
                } else {
                    local_account_data()
                };
                ensure!(
                    account_id_data.insert(account_id, account_data).is_none(),
                    format!(
                        "Budget should not have same account twice: {}",
                        account.name
                    )
                );
            }
        }
        ensure!(
            !all_used_foreign_currencies.is_empty(),
            "No foreign currency accounts were found in the budget (see documentation for how to set up)."
        );
        for account_data in &account_id_data {
            if let (_, AccountData::Foreign { difference_key }) = account_data {
                ensure!(
                    difference_account_ids.contains_key(difference_key),
                    "No difference {} was found",
                    difference_key
                )
            }
        }
        let difference_balances = DifferenceBalances::new(raw_accounts, &account_id_data)?;
        Ok((
            ForeignAccounts {
                all_used_foreign_currencies,
                account_id_data,
                difference_account_ids,
            },
            difference_balances,
        ))
    }

    pub fn get_account_data(&'a self, account_id: &'a YnabAccountId) -> Option<&'a AccountData> {
        self.account_id_data.get(account_id)
    }

    pub fn get_difference_account_id(
        &self,
        difference_key: DifferenceKey,
    ) -> Option<&YnabAccountId> {
        self.difference_account_ids.get(&difference_key)
    }

    pub fn get_all_used_foreign_currencies(&self) -> &HashSet<CurrencyCode> {
        &self.all_used_foreign_currencies
    }

    fn foreign_account_key(account: &ynab_api::models::Account) -> Result<Option<DifferenceKey>> {
        Self::account_difference_key_from_regex(&ACCOUNT_CURRENCY_REGEX, account).chain_err(|| {
            format!(
                "Could not determine foreign currency for account: {}",
                account.name
            )
        })
    }

    fn difference_account_key(
        account: &ynab_api::models::Account,
    ) -> Result<Option<DifferenceKey>> {
        Self::account_difference_key_from_regex(&DIFFERENCE_ACCOUNT_CURRENCY_REGEX, account)
            .chain_err(|| {
                format!(
                    "Could not determine foreign currency for difference account: {}",
                    account.name
                )
            })
    }

    fn account_difference_key_from_regex(
        regex: &Regex,
        account: &ynab_api::models::Account,
    ) -> Result<Option<DifferenceKey>> {
        let currency_code = match Self::account_name_and_note_regex_capture(regex, &account)? {
            Some(currency_code) => currency_code,
            None => return Ok(None),
        };
        let currency = CurrencyCode::from_str(currency_code)
            .expect("Account name and note regex should capture a valid currency code");
        let is_tracking = account._type == ynab_api::models::account::Type::OtherAsset
            || account._type == ynab_api::models::account::Type::OtherLiability;
        Ok(Some(DifferenceKey::new(currency, is_tracking)))
    }

    fn account_name_and_note_regex_capture<'b>(
        regex: &Regex,
        account: &'b ynab_api::models::Account,
    ) -> Result<Option<&'b str>> {
        let mut name_captures_iter = regex.captures_iter(&account.name);
        if let Some(name_captures) = name_captures_iter.next() {
            ensure!(
                name_captures_iter.next().is_none(),
                "Name may not have multiple tags"
            );
            if let Some(note) = &account.note {
                ensure!(
                    regex.captures(&note).is_none(),
                    "Name and note may not both have tags"
                );
            }
            Ok(Some(
                name_captures
                    .get(1)
                    .expect("account_name_and_note_regex_capture regex should have capture group")
                    .as_str(),
            ))
        } else if let Some(note) = &account.note {
            let mut note_captures_iter = regex.captures_iter(note);
            if let Some(note_captures) = note_captures_iter.next() {
                ensure!(
                    note_captures_iter.next().is_none(),
                    "Note may not have multiple tags"
                );
                Ok(Some(
                    note_captures
                        .get(1)
                        .expect(
                            "account_name_and_note_regex_capture regex should have capture group",
                        )
                        .as_str(),
                ))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

impl<'a> DifferenceBalances {
    pub fn new(
        raw_accounts: Vec<ynab_api::models::Account>,
        account_id_data: &HashMap<YnabAccountId, AccountData>,
    ) -> Result<DifferenceBalances> {
        let mut balances = HashMap::new();
        for account in raw_accounts {
            match account_id_data.get(&YnabAccountId::new(account.id)) {
                None => (),
                Some(AccountData::Local { .. }) => (),
                Some(AccountData::Foreign { difference_key }) => {
                    balances
                        .entry(*difference_key)
                        .or_insert_with(ForeignTotalAndDifferenceBalance::new)
                        .foreign_accounts_total += Milliunits::from_scaled_i64(account.balance);
                }
                Some(AccountData::Difference { difference_key }) => {
                    balances
                        .entry(*difference_key)
                        .or_insert_with(ForeignTotalAndDifferenceBalance::new)
                        .difference_account_balance += Milliunits::from_scaled_i64(account.balance);
                }
            }
        }
        Ok(DifferenceBalances { balances })
    }

    pub fn update(
        &mut self,
        difference_key: DifferenceKey,
        transfer_difference_key: Option<DifferenceKey>,
        delta_amount: Milliunits,
    ) {
        self.balances
            .get_mut(&difference_key)
            .unwrap_or_else(|| {
                panic!(
                    "DifferenceBalances should have entry for difference_key: {}",
                    difference_key
                )
            })
            .difference_account_balance += delta_amount;
        if let Some(transfer_difference_key) = transfer_difference_key {
            self.balances
                .get_mut(&transfer_difference_key)
                .unwrap_or_else(|| {
                    panic!(
                        "DifferenceBalances should have entry for transfer_difference_key: {}",
                        transfer_difference_key
                    )
                })
                .foreign_accounts_total -= delta_amount;
        }
    }

    pub fn iter(
        &'a self,
    ) -> Box<dyn Iterator<Item = (&'a DifferenceKey, &'a ForeignTotalAndDifferenceBalance)> + 'a>
    {
        Box::new(self.balances.iter())
    }
}

impl ForeignTotalAndDifferenceBalance {
    fn new() -> ForeignTotalAndDifferenceBalance {
        ForeignTotalAndDifferenceBalance {
            foreign_accounts_total: Milliunits::zero(),
            difference_account_balance: Milliunits::zero(),
        }
    }
}
