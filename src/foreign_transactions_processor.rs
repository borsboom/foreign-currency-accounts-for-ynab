use chrono::NaiveDate;
use log::debug;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::budget_formatter::*;
use crate::constants::*;
use crate::currency_converter_client::*;
use crate::database::models::*;
use crate::database::*;
use crate::errors::*;
use crate::exchange_rates::*;
use crate::foreign_accounts::*;
use crate::import_id_generator::*;
use crate::types::*;
use crate::utilities::*;
use crate::ynab_client::*;

pub struct ForeignTransactionsProcessor<'a> {
    budget_database: &'a BudgetDatabase<'a>,
    ynab_client: &'a YnabBudgetClient<'a>,
    today_date: NaiveDate,
    dry_run: bool,
    auto_approve_transactions: bool,
    auto_approve_adjustments: bool,
    budget_settings: &'a ynab_api::models::BudgetSettings,
    budget_formatter: &'a BudgetFormatter<'a>,
    local_currency: CurrencyCode,
    exchange_rates_cache: ExchangeRatesCache<'a>,
    import_id_generator: ImportIdGenerator,
    foreign_accounts: ForeignAccounts<'a>,
    difference_balances: RefCell<DifferenceBalances>,
}

#[derive(Debug)]
struct TransactionsModificationsData<'a> {
    create_transactions: Vec<ynab_api::models::SaveTransaction>,
    update_transactions: Vec<ynab_api::models::UpdateTransaction>,
    create_import_ids_foreign_ynab_transaction_ids:
        HashMap<YnabImportId<'a>, YnabTransactionId<'a>>,
    delete_difference_transaction_ids: HashSet<YnabTransactionId<'a>>,
}

#[derive(Debug)]
struct ForeignCommonData {
    difference_key: Option<DifferenceKey>,
    transaction_date: NaiveDate,
    transaction_cleared: ynab_api::models::transaction_detail::Cleared,
    transaction_approved: bool,
    transaction_flag_color: Option<ynab_api::models::transaction_detail::FlagColor>,
    transaction_has_matched_transaction_id: bool,
    transaction_has_import_id: bool,
}

#[derive(Debug)]
struct ForeignTransactionData<'a> {
    ynab_transaction_id: &'a YnabTransactionId<'a>,
    payee_id: Option<&'a String>,
    payee_name: Option<&'a str>,
    transfer_account_id: &'a Option<YnabAccountId<'a>>,
    category_id: &'a Option<String>,
    category_name: Option<&'a str>,
    amount: Milliunits,
    memo: &'a Option<String>,
    difference_memo_prefix: &'a str,
    deleted: bool,
}

#[derive(Debug)]
struct TransactionModificationData<'a> {
    prefix: &'a str,
    difference_key: DifferenceKey,
    date: NaiveDate,
    payee_name: Option<&'a str>,
    category_name: Option<&'a str>,
    memo: &'a str,
    amount: Milliunits,
}

impl<'a> ForeignTransactionsProcessor<'a> {
    pub fn run(
        database: &'a Database,
        ynab_client: &'a YnabBudgetClient,
        currency_converter_client: &'a CurrencyConverterClient,
        start_date_arg: Option<NaiveDate>,
        dry_run: bool,
        auto_approve_transactions: bool,
        auto_approve_adjustments: bool,
    ) -> Result<()> {
        let today_date = chrono::Local::today().naive_utc();
        let (initial_budget_state, budget_database) = database
            .get_or_create_budget(ynab_client.budget_id, start_date_arg.unwrap_or(today_date))?;
        ensure!(
            start_date_arg.is_none() || start_date_arg == Some(initial_budget_state.start_date),
            format!(
                "You may not specify a different --{} after the first run for a budget",
                START_DATE_ARG
            )
        );

        println!("Loading latest transactions from YNAB...");
        let transactions_response_data = ynab_client.get_transactions(
            Some(initial_budget_state.start_date),
            initial_budget_state.ynab_server_knowledge,
        )?;
        debug!(
            "Latest transactions received from YNAB: {:#?}",
            &transactions_response_data
        );

        if transactions_response_data.transactions.is_empty()
            && Some(today_date) == initial_budget_state.last_run_date
        {
            println!("No new/updated/deleted transactions; nothing to do!");
            Ok(())
        } else {
            println!("Loading budget settings from YNAB...");
            let budget_settings = ynab_client.get_budget_settings()?;
            let budget_formatter = BudgetFormatter::new(&budget_settings);
            let local_currency = CurrencyCode::from_str(&budget_settings.currency_format.iso_code)?;
            let (foreign_accounts, difference_balances) =
                ForeignAccounts::load(ynab_client, &budget_formatter, local_currency)?;
            ForeignTransactionsProcessor {
                budget_database: &budget_database,
                ynab_client,
                today_date,
                dry_run,
                auto_approve_transactions,
                auto_approve_adjustments,
                budget_settings: &budget_settings,
                budget_formatter: &budget_formatter,
                local_currency,
                exchange_rates_cache: ExchangeRatesCache::new(currency_converter_client, database),
                import_id_generator: ImportIdGenerator::new(),
                foreign_accounts,
                difference_balances: RefCell::new(difference_balances),
            }
            .process(transactions_response_data)
        }
    }

    fn process(
        &self,
        transactions_response_data: ynab_api::models::transactions_response_data::TransactionsResponseData,
    ) -> Result<()> {
        let mut transactions_modifications =
            self.process_transactions(transactions_response_data.transactions)?;
        self.create_adjustments(&mut transactions_modifications)?;
        let update_state_data = self.save_transactions(transactions_modifications)?;
        self.budget_database.update_state(
            transactions_response_data.server_knowledge,
            self.today_date,
            update_state_data,
        )
    }

    fn process_transactions(
        &self,
        latest_transactions: Vec<ynab_api::models::TransactionDetail>,
    ) -> Result<TransactionsModificationsData> {
        println!("Processing latest transactions...");
        let mut transactions_modifications = TransactionsModificationsData::new();
        for parent_transaction in &latest_transactions {
            // If user has deleted a transaction from the difference account,
            // also delete the corresponding database record so that we can't
            // attempt to modify a deleted transaction.
            if parent_transaction.deleted {
                transactions_modifications
                    .delete_difference_transaction_ids
                    .insert(YnabTransactionId::new(parent_transaction.id.clone()));
            }
        }
        for parent_transaction in latest_transactions {
            let parent_transaction_id = YnabTransactionId::new(parent_transaction.id);
            let difference_key = match self
                .foreign_accounts
                .get_account_data(&YnabAccountId::new(parent_transaction.account_id))
            {
                Some(AccountData::Difference { .. }) => {
                    // Don't process transactions that are in difference accounts, since this tool created them.
                    continue;
                }
                Some(AccountData::Foreign { difference_key }) => Some(*difference_key),
                Some(AccountData::Local { .. }) => None,
                None => bail!("Could not find account for transaction"),
            };
            let common_data = ForeignCommonData {
                difference_key,
                transaction_date: parse_iso_date(&parent_transaction.date)?,
                transaction_cleared: parent_transaction.cleared,
                transaction_approved: parent_transaction.approved,
                transaction_flag_color: parent_transaction.flag_color,
                transaction_has_matched_transaction_id: parent_transaction
                    .matched_transaction_id
                    .is_some(),
                transaction_has_import_id: parent_transaction.import_id.is_some(),
            };
            for (subtransaction_index, subtransaction) in
                parent_transaction.subtransactions.iter().enumerate()
            {
                self.process_parent_or_subtransaction(
                    &mut transactions_modifications,
                    &common_data,
                    &ForeignTransactionData {
                        ynab_transaction_id: &YnabTransactionId::new(&subtransaction.id),
                        payee_id: {
                            #[allow(clippy::or_fun_call)]
                            subtransaction
                                .payee_id
                                .as_ref()
                                .or(parent_transaction.payee_id.as_ref())
                        },
                        payee_name: None,
                        category_id: &subtransaction.category_id,
                        category_name: None,
                        amount: Milliunits::from_scaled_i64(subtransaction.amount),
                        memo: &subtransaction.memo,
                        transfer_account_id: &subtransaction
                            .transfer_account_id
                            .clone()
                            .map(YnabAccountId::new),
                        difference_memo_prefix: &format!(
                            " (split {}/{})",
                            subtransaction_index + 1,
                            parent_transaction.subtransactions.len()
                        ),
                        deleted: subtransaction.deleted || parent_transaction.deleted,
                    },
                )?;
            }
            self.process_parent_or_subtransaction(
                &mut transactions_modifications,
                &common_data,
                &ForeignTransactionData {
                    ynab_transaction_id: &parent_transaction_id,
                    payee_id: parent_transaction.payee_id.as_ref(),
                    payee_name: parent_transaction.payee_name.as_ref().map(|s| s.as_str()),
                    transfer_account_id: &parent_transaction
                        .transfer_account_id
                        .map(YnabAccountId::new),
                    category_id: &parent_transaction.category_id,
                    category_name: parent_transaction
                        .category_name
                        .as_ref()
                        .map(|s| s.as_str()),
                    amount: Milliunits::from_scaled_i64(parent_transaction.amount),
                    memo: &parent_transaction.memo,
                    difference_memo_prefix: "",
                    // If there are subtransactions, we create those in the
                    // difference account instead of the parent transaction, so
                    // we consider the parent "deleted."
                    deleted: parent_transaction.deleted
                        || !parent_transaction.subtransactions.is_empty(),
                },
            )?;
        }
        Ok(transactions_modifications)
    }

    fn process_parent_or_subtransaction(
        &self,
        transactions_modifications: &mut TransactionsModificationsData,
        common_data: &ForeignCommonData,
        foreign_data: &ForeignTransactionData,
    ) -> Result<()> {
        let (difference_memo_suffix, force_convert, force_no_convert) = match foreign_data.memo {
            None => ("".to_string(), false, false),
            Some(memo) => (
                format!(" {}", memo),
                FORCE_CONVERT_REGEX.is_match(memo),
                FORCE_NO_CONVERT_REGEX.is_match(memo),
            ),
        };
        let convert_transfer_account = match foreign_data.transfer_account_id {
            Some(account_id) => match self.foreign_accounts.get_account_data(account_id) {
                Some(AccountData::Local { force_convert }) => *force_convert,
                _ => false,
            },
            None => false,
        };
        let (difference_amount, difference_memo) = if foreign_data.deleted
            || force_no_convert
            || (foreign_data.transfer_account_id.is_some()
                && !convert_transfer_account
                && !force_convert)
            || (common_data.transaction_has_matched_transaction_id
                && common_data.transaction_has_import_id
                && !common_data.transaction_approved)
        {
            // The YNAB API does not support deleting a transaction, so instead
            // we update the difference transaction to a zero amount.
            (
                Milliunits::zero(),
                format!(
                    "<DELETED>{}{}",
                    foreign_data.difference_memo_prefix, difference_memo_suffix
                ),
            )
        } else if let Some(difference_key) = common_data.difference_key {
            let exchange_rate = self.get_transaction_date_exchange_rate(
                difference_key.currency,
                common_data.transaction_date,
            )?;
            (
                self.round_to_budget_decimal_digits(
                    foreign_data.amount.convert_currency(exchange_rate) - foreign_data.amount,
                ),
                format!(
                    "<{}>{}{}",
                    self.format_exchange(
                        difference_key.currency,
                        foreign_data.amount,
                        exchange_rate
                    ),
                    foreign_data.difference_memo_prefix,
                    difference_memo_suffix
                ),
            )
        } else {
            (
                Milliunits::zero(),
                format!(
                    "<MOVED TO LOCAL CURRENCY ACCOUNT>{}{}",
                    foreign_data.difference_memo_prefix, difference_memo_suffix
                ),
            )
        };
        let opt_existing_difference_transaction = self
            .budget_database
            .get_difference_transaction_by_foreign_id(&foreign_data.ynab_transaction_id)?
            .and_then(|existing_difference_transaction| {
                if transactions_modifications
                    .delete_difference_transaction_ids
                    .contains(&existing_difference_transaction.difference_transaction_id)
                {
                    None
                } else {
                    Some(existing_difference_transaction)
                }
            });
        let mut difference_balances = self.difference_balances.borrow_mut();
        if let Some(old_difference_transaction) = &opt_existing_difference_transaction {
            difference_balances.update(
                old_difference_transaction.difference_key,
                old_difference_transaction.transfer_key,
                -old_difference_transaction.amount,
            );
        }
        let opt_difference_key = match (
            common_data.difference_key,
            &opt_existing_difference_transaction,
        ) {
            (Some(difference_key), _) => Some(difference_key),
            (None, Some(existing_difference_transaction)) => {
                Some(existing_difference_transaction.difference_key)
            }
            (None, None) => None,
        };
        if let Some(difference_key) = opt_difference_key {
            let difference_account_id = self
                .foreign_accounts
                .get_difference_account_id(difference_key)
                .expect("Difference account should exist");
            if let Some(difference_transaction) = &opt_existing_difference_transaction {
                self.print_transaction_modification(&TransactionModificationData {
                    prefix: "Update difference",
                    difference_key,
                    date: common_data.transaction_date,
                    payee_name: foreign_data.payee_name,
                    category_name: foreign_data.category_name,
                    memo: &difference_memo,
                    amount: difference_amount,
                });
                transactions_modifications.update_transactions.push(
                    ynab_api::models::UpdateTransaction {
                        id: difference_transaction.difference_transaction_id.to_string(),
                        account_id: difference_account_id.to_string(),
                        date: format_iso_date(common_data.transaction_date),
                        amount: difference_amount.to_scaled_i64(),
                        payee_id: foreign_data.payee_id.cloned(),
                        payee_name: None,
                        category_id: foreign_data.category_id.clone(),
                        memo: Some(difference_memo),
                        cleared: Some(transaction_detail_cleared_to_update_transaction(
                            common_data.transaction_cleared,
                        )),
                        approved: None,
                        flag_color: common_data
                            .transaction_flag_color
                            .map(transaction_detail_flag_color_to_update_transaction),
                        import_id: None,
                    },
                );
            } else if !difference_amount.is_zero() {
                let difference_import_id = self.import_id_generator.next_import_id();
                transactions_modifications
                    .create_import_ids_foreign_ynab_transaction_ids
                    .insert(
                        YnabImportId::new(difference_import_id.raw.clone().into_owned()),
                        YnabTransactionId::new(
                            foreign_data.ynab_transaction_id.raw.clone().into_owned(),
                        ),
                    );
                self.print_transaction_modification(&TransactionModificationData {
                    prefix: "Create difference",
                    difference_key,
                    date: common_data.transaction_date,
                    payee_name: foreign_data.payee_name,
                    category_name: foreign_data.category_name,
                    memo: &difference_memo,
                    amount: difference_amount,
                });
                transactions_modifications.create_transactions.push(
                    ynab_api::models::SaveTransaction {
                        account_id: difference_account_id.to_string(),
                        date: format_iso_date(common_data.transaction_date),
                        amount: difference_amount.to_scaled_i64(),
                        payee_id: foreign_data.payee_id.cloned(),
                        payee_name: None,
                        category_id: foreign_data.category_id.clone(),
                        memo: Some(difference_memo),
                        cleared: Some(transaction_detail_cleared_to_save_transaction(
                            common_data.transaction_cleared,
                        )),
                        approved: Some(self.auto_approve_transactions),
                        flag_color: common_data
                            .transaction_flag_color
                            .map(transaction_detail_flag_color_to_save_transaction),
                        import_id: Some(difference_import_id.raw.into()),
                    },
                );
            }
            let transfer_difference_key =
                foreign_data.transfer_account_id.as_ref().and_then(|id| {
                    match self.foreign_accounts.get_account_data(id) {
                        Some(AccountData::Foreign { difference_key }) => Some(*difference_key),
                        _ => None,
                    }
                });
            difference_balances.update(difference_key, transfer_difference_key, difference_amount);
        } else {
            assert!(
                difference_amount.is_zero(),
                "difference_amount should be 0 if transaction has difference_key is None"
            );
        }

        Ok(())
    }

    fn create_adjustments(
        &self,
        transactions_modifications: &mut TransactionsModificationsData,
    ) -> Result<()> {
        println!("Checking for adjustments...");
        let difference_balances = self.difference_balances.borrow();
        for (&difference_key, foreign_total_and_difference_balance) in difference_balances.iter() {
            if let Some(difference_account_id) = self
                .foreign_accounts
                .get_difference_account_id(difference_key)
            {
                let exchange_rate = self
                    .get_transaction_date_exchange_rate(difference_key.currency, self.today_date)?;
                let expected_difference_account_balance = self.round_to_budget_decimal_digits(
                    foreign_total_and_difference_balance
                        .foreign_accounts_total
                        .convert_currency(exchange_rate)
                        - foreign_total_and_difference_balance.foreign_accounts_total,
                );
                let difference_adjustment_amount = expected_difference_account_balance
                    - foreign_total_and_difference_balance.difference_account_balance;
                if difference_adjustment_amount != Milliunits::zero() {
                    let adjustment_payee_name = format_adjustment_payee_name(difference_key);
                    let adjustment_memo = format!(
                        "{}{}",
                        ADJUSTMENT_MEMO_PREFIX,
                        self.format_exchange(
                            difference_key.currency,
                            foreign_total_and_difference_balance.foreign_accounts_total,
                            exchange_rate
                        )
                    );
                    self.print_transaction_modification(&TransactionModificationData {
                        prefix: "Create adjustment",
                        difference_key,
                        date: self.today_date,
                        payee_name: Some(&adjustment_payee_name),
                        category_name: None,
                        memo: &adjustment_memo,
                        amount: difference_adjustment_amount,
                    });
                    transactions_modifications.create_transactions.push(
                        ynab_api::models::SaveTransaction {
                            account_id: difference_account_id.to_string(),
                            date: format_iso_date(self.today_date),
                            amount: difference_adjustment_amount.to_scaled_i64(),
                            payee_id: None,
                            payee_name: Some(adjustment_payee_name),
                            category_id: None,
                            memo: Some(adjustment_memo),
                            cleared: None,
                            approved: Some(self.auto_approve_adjustments),
                            flag_color: None,
                            import_id: Some(self.import_id_generator.next_import_id().to_string()),
                        },
                    );
                }
            }
        }
        Ok(())
    }

    fn save_transactions<'b>(
        &self,
        transactions_modifications: TransactionsModificationsData<'b>,
    ) -> Result<UpdateBudgetState<'b>> {
        let mut database_create_difference_transactions = Vec::new();
        let mut database_update_difference_transactions = Vec::new();
        let had_changes = if transactions_modifications.has_changes() {
            println!("No new/changed difference transactions; nothing to do!");
            false
        } else {
            debug!(
                "Changed transactions to save to YNAB: {:#?}",
                transactions_modifications.update_transactions
            );
            if !transactions_modifications.update_transactions.is_empty() && !self.dry_run {
                println!("Saving changed transactions to YNAB...");
                let updated_transactions = self
                    .ynab_client
                    .update_transactions(transactions_modifications.update_transactions)?;
                debug!(
                    "Response from YNAB after saving changed transactions: {:#?}",
                    updated_transactions
                );
                for updated_transaction in updated_transactions {
                    let updated_transaction_id = YnabTransactionId::new(updated_transaction.id);
                    database_update_difference_transactions.push(DifferenceTransaction {
                        difference_transaction_id: updated_transaction_id,
                        amount: Milliunits::from_scaled_i64(updated_transaction.amount),
                        difference_key: self.difference_account_key_for_save(&YnabAccountId::new(
                            updated_transaction.account_id,
                        )),
                        transfer_key: updated_transaction.transfer_account_id.and_then(|a| {
                            self.transfer_account_key_for_save(&YnabAccountId::new(a))
                        }),
                    });
                }
            }
            debug!(
                "New transactions to save to YNAB: {:#?}",
                transactions_modifications.create_transactions
            );
            if !transactions_modifications.create_transactions.is_empty() && !self.dry_run {
                println!("Saving new transactions to YNAB...");
                let created_transactions = self
                    .ynab_client
                    .create_transactions(transactions_modifications.create_transactions)?;
                debug!(
                    "Response from YNAB after saving new transactions: {:#?}",
                    created_transactions
                );
                for created_transaction in created_transactions {
                    if let Some(import_id) = created_transaction.import_id {
                        if let Some(foreign_ynab_transaction_id) = transactions_modifications
                            .create_import_ids_foreign_ynab_transaction_ids
                            .get(&YnabImportId::new(import_id))
                        {
                            database_create_difference_transactions.push(
                                CreateDifferenceTransaction {
                                    foreign_transaction_id: foreign_ynab_transaction_id.clone(),
                                    inner: DifferenceTransaction {
                                        difference_transaction_id: YnabTransactionId::new(
                                            created_transaction.id,
                                        ),
                                        amount: Milliunits::from_scaled_i64(
                                            created_transaction.amount,
                                        ),
                                        difference_key: self.difference_account_key_for_save(
                                            &YnabAccountId::new(created_transaction.account_id),
                                        ),
                                        transfer_key: created_transaction
                                            .transfer_account_id
                                            .and_then(|a| {
                                                self.transfer_account_key_for_save(
                                                    &YnabAccountId::new(a),
                                                )
                                            }),
                                    },
                                },
                            );
                        }
                    }
                }
            }
            if self.dry_run {
                println!("\nNOTE: No transactions were actually saved.");
                println!("Re-run with '--yes' to save the changes to YNAB.");
            } else {
                println!("Done!");
            }
            true
        };
        Ok(UpdateBudgetState {
            had_changes,
            create_difference_transactions: database_create_difference_transactions,
            update_difference_transactions: database_update_difference_transactions,
            delete_difference_transaction_ids: transactions_modifications
                .delete_difference_transaction_ids,
        })
    }

    fn difference_account_key_for_save(&self, account_id: &YnabAccountId) -> DifferenceKey {
        match self.foreign_accounts.get_account_data(account_id) {
            Some(AccountData::Difference { difference_key }) => *difference_key,
            _ => panic!("New/changed transaction's account should be a difference account"),
        }
    }

    fn transfer_account_key_for_save(&self, account_id: &YnabAccountId) -> Option<DifferenceKey> {
        match self.foreign_accounts.get_account_data(account_id) {
            Some(AccountData::Foreign { difference_key }) => Some(*difference_key),
            _ => None,
        }
    }

    fn get_transaction_date_exchange_rate(
        &self,
        from_currency: CurrencyCode,
        date: NaiveDate,
    ) -> Result<ExchangeRate> {
        self.exchange_rates_cache.get_exchange_rate(
            self.foreign_accounts.get_all_used_foreign_currencies(),
            from_currency,
            self.local_currency,
            date,
        )
    }

    fn print_transaction_modification(&self, data: &TransactionModificationData) {
        println!("  {} transaction:", data.prefix,);
        println!("     Account: Difference {}", data.difference_key);
        println!(
            "        Date: {}",
            self.budget_formatter.format_date(data.date)
        );
        if let Some(payee_name) = data.payee_name {
            println!("       Payee: {}", payee_name);
        }
        if let Some(category) = data.category_name {
            println!("    Category: {}", category);
        }
        println!("        Memo: {}", data.memo);
        println!(
            "      Amount: {}",
            self.budget_formatter.format_milliunits(data.amount)
        )
    }

    fn format_exchange(
        &self,
        currency: CurrencyCode,
        amount: Milliunits,
        exchange_rate: ExchangeRate,
    ) -> String {
        format!(
            "{} @{}/{} = {}",
            self.budget_formatter
                .format_milliunits_with_code(currency, amount),
            self.budget_formatter.format_exchange_rate(exchange_rate),
            currency,
            self.budget_formatter
                .format_milliunits(amount.convert_currency(exchange_rate)),
        )
    }

    fn round_to_budget_decimal_digits(&self, amount: Milliunits) -> Milliunits {
        amount.round_bankers(self.budget_settings.currency_format.decimal_digits as u32)
    }
}

impl<'a> TransactionsModificationsData<'a> {
    pub fn new() -> TransactionsModificationsData<'a> {
        TransactionsModificationsData {
            create_transactions: Vec::new(),
            update_transactions: Vec::new(),
            create_import_ids_foreign_ynab_transaction_ids: HashMap::new(),
            delete_difference_transaction_ids: HashSet::new(),
        }
    }

    pub fn has_changes(&self) -> bool {
        self.create_transactions.is_empty() && self.update_transactions.is_empty()
    }
}
