pub mod models;

use chrono::{Datelike, NaiveDate};
use diesel::prelude::*;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::{fs, path};

use crate::database::models::*;
use crate::errors::*;
use crate::schema;
use crate::types::*;

embed_migrations!("migrations");

pub struct Database {
    connection: SqliteConnection,
    dry_run: bool,
}

pub struct BudgetDatabase<'a> {
    connection: &'a SqliteConnection,
    run_state: BudgetRunState,
}

#[derive(Debug)]
enum BudgetRunState {
    DryRun(Option<i32>),
    Live(i32),
}

impl Database {
    pub fn establish_connection(database_file: &str, dry_run: bool) -> Result<Database> {
        let parent = path::Path::new(database_file).parent().chain_err(|| {
            format!(
                "Failed to determine parent directory of database file path: {}",
                database_file
            )
        })?;
        fs::create_dir_all(parent)
            .chain_err(|| format!("Failed to create database directory: {}", parent.display()))?;
        debug!("Using database file: {}", database_file);
        let connection = SqliteConnection::establish(&database_file)
            .chain_err(|| "Failed to establish SQLite database connection")?;
        embedded_migrations::run(&connection)
            .chain_err(|| "Failed to perform database schema migrations")?;
        Ok(Database {
            connection,
            dry_run,
        })
    }

    pub fn get_exchange_rate(
        &self,
        from_currency: CurrencyCode,
        to_currency: CurrencyCode,
        date_: NaiveDate,
    ) -> Result<Option<ExchangeRate>> {
        use schema::exchange_rates::dsl::*;
        schema::exchange_rates::table
            .select(exchange_rate)
            .filter(from_currency_code.eq(from_currency.to_str()))
            .filter(to_currency_code.eq(to_currency.to_str()))
            .filter(date.eq(date_.num_days_from_ce()))
            .first::<i64>(&self.connection)
            .optional()
            .chain_err(|| "Failed to load exchange rate from database")
            .map(|res| res.map(ExchangeRate::from_scaled_i64))
    }

    pub fn get_known_exchange_rates(
        &self,
        from_currencies: &HashSet<CurrencyCode>,
        to_currency: CurrencyCode,
        date_: NaiveDate,
    ) -> Result<HashMap<CurrencyCode, ExchangeRate>> {
        use schema::exchange_rates::dsl::*;
        schema::exchange_rates::table
            .select((from_currency_code, exchange_rate))
            .filter(from_currency_code.eq_any(from_currencies.iter().map(|cur| cur.to_str())))
            .filter(to_currency_code.eq(to_currency.to_str()))
            .filter(date.eq(date_.num_days_from_ce()))
            .load::<(String, i64)>(&self.connection)
            .chain_err(|| "Failed to load exchange rates from database")?
            .into_iter()
            .map(|(currency, rate)| {
                CurrencyCode::from_str(&currency)
                    .map(|code| (code, ExchangeRate::from_scaled_i64(rate)))
            })
            .collect::<Result<_>>()
    }

    pub fn create_exchange_rate(
        &self,
        from_currency: CurrencyCode,
        to_currency: CurrencyCode,
        date_: NaiveDate,
        exchange_rate_: ExchangeRate,
    ) -> Result<()> {
        use schema::exchange_rates::dsl::*;
        diesel::insert_into(schema::exchange_rates::table)
            .values((
                date.eq(date_.num_days_from_ce()),
                from_currency_code.eq(from_currency.to_str()),
                to_currency_code.eq(to_currency.to_str()),
                exchange_rate.eq(exchange_rate_.to_scaled_i64()),
            ))
            .execute(&self.connection)
            .chain_err(|| "Failed to save exchange rate to database")?;
        Ok(())
    }

    pub fn get_or_create_budget<'a>(
        &'a self,
        ynab_budget_id_: &'a str,
        default_start_date: NaiveDate,
    ) -> Result<(BudgetState, BudgetDatabase<'a>)> {
        if let Some(result) = self.get_budget(ynab_budget_id_)? {
            Ok(result)
        } else {
            let budget_db = self.create_budget(ynab_budget_id_, default_start_date)?;
            Ok((
                BudgetState {
                    start_date: default_start_date,
                    ynab_server_knowledge: None,
                    last_run_date: None,
                },
                budget_db,
            ))
        }
    }

    fn create_budget<'a>(
        &'a self,
        ynab_budget_id_: &'a str,
        start_date_: NaiveDate,
    ) -> Result<BudgetDatabase<'a>> {
        if self.dry_run {
            return Ok(BudgetDatabase {
                connection: &self.connection,
                run_state: BudgetRunState::DryRun(None),
            });
        }
        use schema::budgets::dsl::*;
        diesel::insert_into(schema::budgets::table)
            .values((
                ynab_budget_id.eq(ynab_budget_id_),
                start_date.eq(start_date_.num_days_from_ce()),
            ))
            .execute(&self.connection)
            .chain_err(|| "Failed save new budget state to database")?;
        let db_budget_id = schema::budgets::table
            .select(id)
            .filter(ynab_budget_id.eq(ynab_budget_id))
            .first(&self.connection)
            .chain_err(|| "Failed to read budget state record ID from database")?;
        Ok(BudgetDatabase {
            connection: &self.connection,
            run_state: BudgetRunState::Live(db_budget_id),
        })
    }

    fn get_budget<'a>(
        &'a self,
        ynab_budget_id_: &'a str,
    ) -> Result<Option<(BudgetState, BudgetDatabase<'a>)>> {
        use schema::budgets::dsl::*;
        if let Some((db_budget_id, start_days_from_ce, knowledge, last_run_days_from_ce)) =
            schema::budgets::table
                .select((id, start_date, ynab_server_knowledge, last_run_date))
                .filter(ynab_budget_id.eq(ynab_budget_id_))
                .first::<(i32, i32, Option<i64>, Option<i32>)>(&self.connection)
                .optional()
                .chain_err(|| "Failed to load budget state from database")?
        {
            Ok(Some((
                BudgetState {
                    start_date: NaiveDate::from_num_days_from_ce(start_days_from_ce),
                    ynab_server_knowledge: knowledge,
                    last_run_date: last_run_days_from_ce.map(NaiveDate::from_num_days_from_ce),
                },
                BudgetDatabase {
                    connection: &self.connection,
                    run_state: if self.dry_run {
                        BudgetRunState::DryRun(Some(db_budget_id))
                    } else {
                        BudgetRunState::Live(db_budget_id)
                    },
                },
            )))
        } else {
            Ok(None)
        }
    }
}

impl<'a> BudgetDatabase<'a> {
    pub fn update_state(
        &self,
        ynab_server_knowledge: i64,
        last_run_date: NaiveDate,
        update_state: UpdateBudgetState,
    ) -> Result<()> {
        let opt_db_budget_id = if update_state.had_changes {
            self.run_state.live_database_budget_id()
        } else {
            self.run_state.dry_run_database_budget_id()
        };
        if let Some(db_budget_id) = opt_db_budget_id {
            self.connection
                .transaction(|| {
                    // Must delete before creating, otherwise when we insert we
                    // might violate a unique constraint.
                    self.delete_difference_transactions(
                        db_budget_id,
                        update_state.delete_difference_transaction_ids,
                    )?;
                    self.create_difference_transactions(
                        db_budget_id,
                        &update_state.create_difference_transactions,
                    )?;
                    self.update_difference_transactions(
                        db_budget_id,
                        &update_state.update_difference_transactions,
                    )?;
                    self.update_budget(db_budget_id, ynab_server_knowledge, last_run_date)
                })
                .chain_err(|| "Failed to save budget state in database")
        } else {
            Ok(())
        }
    }

    pub fn get_difference_transaction_by_foreign_id(
        &self,
        foreign_ynab_transaction_id_: &YnabTransactionId,
    ) -> Result<Option<DifferenceTransaction>> {
        if let Some(db_budget_id) = self.run_state.dry_run_database_budget_id() {
            use schema::difference_transactions::dsl::*;
            schema::difference_transactions::table
                .select((difference_ynab_transaction_id,
                         difference_amount_milliunits,
                         difference_currency_code,
                         difference_account_class,
                         transfer_currency_code,
                         transfer_account_class))
                .filter(budget_id.eq(db_budget_id))
                .filter(foreign_ynab_transaction_id.eq(&foreign_ynab_transaction_id_.raw))
                .first::<(String, i64, String, String, Option<String>, Option<String>)>(self.connection)
                .optional()
                .map(|opt| {
                    opt.map(|(difference_transaction_id,
                              amount,
                              difference_currency_code_,
                              difference_account_class_,
                              transfer_currency_code_,
                              transfer_account_class_)| DifferenceTransaction {
                        difference_transaction_id: YnabTransactionId::new(difference_transaction_id),
                        amount: Milliunits::from_scaled_i64(amount),
                        difference_key: DifferenceKey {
                            currency: CurrencyCode::from_str(&difference_currency_code_)
                                .expect("difference_transactions.difference_currency_code should be valid currency code"),
                            account_class: account_class_from_str(&difference_account_class_)
                                .expect("difference_transactions.difference_account_class should be valid character"),
                        },
                        transfer_key: transfer_currency_code_.map(|code| DifferenceKey {
                            currency: CurrencyCode::from_str(&code)
                                .expect("difference_transactions.transfer_currency_code should be valid currency code"),
                            account_class: account_class_from_str(&transfer_account_class_
                                .expect("difference_transactions.transfer_account_class should not be null when transfer_currency_code is non-null"))
                                .expect("difference_transactions.transfer_account_class should be a valid character"),
                        }),
                    })
                })
                .chain_err(|| "Failed to load existing difference transaction from database")
        } else {
            Ok(None)
        }
    }

    fn update_budget(
        &self,
        db_budget_id: i32,
        ynab_server_knowledge_: i64,
        last_run_date_: NaiveDate,
    ) -> QueryResult<()> {
        use schema::budgets::dsl::*;
        diesel::update(schema::budgets::table.filter(id.eq(db_budget_id)))
            .set((
                ynab_server_knowledge.eq(Some(ynab_server_knowledge_)),
                last_run_date.eq(Some(last_run_date_.num_days_from_ce())),
            ))
            .execute(self.connection)?;
        Ok(())
    }

    fn delete_difference_transactions(
        &self,
        db_budget_id: i32,
        delete_difference_transaction_ids: HashSet<YnabTransactionId>,
    ) -> QueryResult<()> {
        use schema::difference_transactions::dsl::*;
        diesel::delete(schema::difference_transactions::table)
            .filter(budget_id.eq(db_budget_id))
            .filter(
                difference_ynab_transaction_id
                    .eq_any(delete_difference_transaction_ids.into_iter().map(|v| v.raw)),
            )
            .execute(self.connection)?;
        Ok(())
    }

    fn create_difference_transactions(
        &self,
        db_budget_id: i32,
        transactions: &[CreateDifferenceTransaction],
    ) -> QueryResult<()> {
        use schema::difference_transactions::dsl::*;
        for transaction in transactions {
            diesel::insert_into(schema::difference_transactions::table)
                .values((
                    budget_id.eq(db_budget_id),
                    foreign_ynab_transaction_id.eq(&transaction.foreign_transaction_id.raw),
                    difference_ynab_transaction_id
                        .eq(&transaction.inner.difference_transaction_id.raw),
                    difference_amount_milliunits.eq(transaction.inner.amount.to_scaled_i64()),
                    difference_currency_code.eq(transaction.inner.difference_key.currency.to_str()),
                    difference_account_class.eq(account_class_to_str(
                        transaction.inner.difference_key.account_class,
                    )),
                    transfer_currency_code.eq(transaction
                        .inner
                        .transfer_key
                        .as_ref()
                        .map(|k| k.currency.to_str())),
                    transfer_account_class.eq(transaction
                        .inner
                        .transfer_key
                        .map(|k| account_class_to_str(k.account_class))),
                ))
                .execute(self.connection)?;
        }
        Ok(())
    }

    fn update_difference_transactions(
        &self,
        db_budget_id: i32,
        transactions: &[DifferenceTransaction],
    ) -> QueryResult<()> {
        use schema::difference_transactions::dsl::*;
        for transaction in transactions {
            diesel::update(schema::difference_transactions::table)
                .filter(budget_id.eq(db_budget_id))
                .filter(
                    difference_ynab_transaction_id.eq(&transaction.difference_transaction_id.raw),
                )
                .set((
                    difference_amount_milliunits.eq(transaction.amount.to_scaled_i64()),
                    difference_currency_code.eq(transaction.difference_key.currency.to_str()),
                    difference_account_class.eq(account_class_to_str(
                        transaction.difference_key.account_class,
                    )),
                    transfer_currency_code.eq(transaction
                        .transfer_key
                        .as_ref()
                        .map(|k| k.currency.to_str())),
                    transfer_account_class.eq(transaction
                        .transfer_key
                        .map(|k| account_class_to_str(k.account_class))),
                ))
                .execute(self.connection)?;
        }
        Ok(())
    }
}

impl BudgetRunState {
    fn live_database_budget_id(&self) -> Option<i32> {
        match self {
            BudgetRunState::DryRun(_) => None,
            BudgetRunState::Live(id) => Some(*id),
        }
    }

    fn dry_run_database_budget_id(&self) -> Option<i32> {
        match self {
            BudgetRunState::DryRun(option_id) => *option_id,
            BudgetRunState::Live(id) => Some(*id),
        }
    }
}

fn account_class_to_str(value: AccountClass) -> &'static str {
    match value {
        AccountClass::Debit => "D",
        AccountClass::Credit => "C",
        AccountClass::Tracking => "T",
    }
}

fn account_class_from_str(value: &str) -> Option<AccountClass> {
    match value {
        "D" => Some(AccountClass::Debit),
        "C" => Some(AccountClass::Credit),
        "T" => Some(AccountClass::Tracking),
        _ => None,
    }
}
