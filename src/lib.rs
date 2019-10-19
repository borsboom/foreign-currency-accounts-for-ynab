#![warn(clippy::all)]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate error_chain;

mod budget_formatter;
mod cli;
mod constants;
mod currency_converter_client;
mod database;
mod exchange_rates;
mod foreign_accounts;
mod foreign_accounts_processor;
mod import_id_generator;
mod schema;
mod types;
mod utilities;
mod ynab_client;

mod errors {
    error_chain! {}
}

pub use cli::run;
