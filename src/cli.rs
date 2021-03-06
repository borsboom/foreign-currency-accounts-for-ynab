use log::debug;
use std::ffi::OsStr;
use std::{env, result, str, string};

use crate::constants::*;
use crate::currency_converter_client::*;
use crate::database::*;
use crate::errors::*;
use crate::foreign_transactions_processor::*;
use crate::utilities::*;
use crate::ynab_client::*;

pub fn run() -> Result<()> {
    initialize()?;
    run_clap_matches(get_clap_matches())
}

fn initialize() -> Result<()> {
    openssl_probe::init_ssl_cert_env_vars();

    let proj_dirs = directories::ProjectDirs::from("io", "borsboom", clap::crate_name!())
        .chain_err(|| "Failed to determine user data directory")?;
    let mut configuration_file = proj_dirs.config_dir().to_path_buf();
    configuration_file.push("env");
    let mut default_database_file = proj_dirs.data_dir().to_path_buf();
    default_database_file.push(DEFAULT_DATABASE_FILENAME);

    dotenv::dotenv().ok();
    dotenv::from_path(&configuration_file).ok();
    default_env(DATABASE_FILE_ENV, default_database_file);
    default_env(AUTO_APPROVE_TRANSACTIONS_ENV, false.to_string());
    default_env(AUTO_APPROVE_ADJUSTMENTS_ENV, false.to_string());
    default_env(
        CURRENCY_CONVERTER_API_BASE_URL_ENV,
        DEFAULT_CURRENCY_CONVERTER_API_BASE_URL,
    );
    default_env(
        CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST_ENV,
        DEFAULT_CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST.to_string(),
    );

    env_logger::init();
    debug!("Using configuration file path: {:?}", configuration_file);

    Ok(())
}

fn get_clap_matches() -> clap::ArgMatches<'static> {
    clap::App::new(clap::crate_name!())
        .version(option_env!("CI_BUILD_VERSION").unwrap_or(clap::crate_version!()))
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(
            clap::Arg::with_name(YES_ARG)
                .long(YES_ARG)
                .short("y")
                .help("Save changes to YNAB budget and database (without this, runs in \"dry run\" mode)"))
        .arg(
            clap::Arg::with_name(AUTO_APPROVE_TRANSACTIONS_ARG)
                .env(AUTO_APPROVE_TRANSACTIONS_ENV)
                .long(AUTO_APPROVE_TRANSACTIONS_ARG)
                .value_name("BOOLEAN")
                .help("Automatically approve new difference transactions")
                .takes_value(true)
                .possible_values(&POSSIBLE_BOOL_VALUES),
        )
        .arg(
            clap::Arg::with_name(AUTO_APPROVE_ADJUSTMENTS_ARG)
                .env(AUTO_APPROVE_ADJUSTMENTS_ENV)
                .long(AUTO_APPROVE_ADJUSTMENTS_ARG)
                .value_name("BOOLEAN")
                .help("Automatically approve exchange rate adjustment transactions")
                .takes_value(true)
                .possible_values(&POSSIBLE_BOOL_VALUES),
        )
        .arg(
            clap::Arg::with_name(YNAB_ACCESS_TOKEN_ARG)
                .env(YNAB_ACCESS_TOKEN_ENV)
                .long(YNAB_ACCESS_TOKEN_ARG)
                .value_name("TOKEN")
                .help("YNAB personal access token (see documentation for setup)")
                .takes_value(true)
                .required(true),
        )
        .arg(
            clap::Arg::with_name(CURRENCY_CONVERTER_API_KEY_ARG)
                .env(CURRENCY_CONVERTER_API_KEY_ENV)
                .long(CURRENCY_CONVERTER_API_KEY_ARG)
                .value_name("KEY")
                .help("Currency Converter API key (see documentation for setup)")
                .takes_value(true)
                .required(true),
        )
        .arg(
            clap::Arg::with_name(CURRENCY_CONVERTER_API_BASE_URL_ARG)
                .env(CURRENCY_CONVERTER_API_BASE_URL_ENV)
                .long(CURRENCY_CONVERTER_API_BASE_URL_ARG)
                .value_name("URL")
                .help("Currency Converter API base URL, without trailing '/' (to use non-free version)")
                .takes_value(true)
                .required(true),
        )
        .arg(
            clap::Arg::with_name(CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST_ARG)
                .env(CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST_ENV)
                .long(CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST_ARG)
                .value_name("NUMBER")
                .help("Maximum number of currency pairs allowed per Currency Converter API request (provisioned when you register for non-free API)")
                .takes_value(true)
                .validator(|value| map_validator(value.parse::<usize>())),
        )
        .arg(
            clap::Arg::with_name(YNAB_BUDGET_ID_ARG)
                .env(YNAB_BUDGET_ID_ENV)
                .long(YNAB_BUDGET_ID_ARG)
                .value_name("ID")
                .help("YNAB budget identifier (see documentation for setup)")
                .takes_value(true)
                .required(true),
        )
        .arg(
            clap::Arg::with_name(START_DATE_ARG)
                .long(START_DATE_ARG)
                .value_name("YYYY-MM-DD")
                .help("Transactions from this date will be processed.  Defaults to thirty days prior to today's date.  May only be set on first run for the budget.")
                .takes_value(true)
                .validator(|value| map_validator(parse_iso_date(&value))
                ),
        )
        .arg(
            clap::Arg::with_name(DATABASE_FILE_ARG)
                .env(DATABASE_FILE_ENV)
                .long(DATABASE_FILE_ARG)
                .value_name("PATH")
                .help("Set the database file where local data will be stored")
                .takes_value(true),
        )
        .get_matches()
}

fn map_validator<T, U>(result: result::Result<T, U>) -> result::Result<(), String>
where
    U: string::ToString,
{
    result.map(|_| ()).map_err(|err| err.to_string())
}

fn run_clap_matches(matches: clap::ArgMatches) -> Result<()> {
    let dry_run = !matches.is_present(YES_ARG);
    let auto_approve_transactions =
        clap::value_t!(matches.value_of(AUTO_APPROVE_TRANSACTIONS_ARG), bool)
            .expect("CLAP matches should have valid AUTO_APPROVE_TRANSACTIONS_ARG");
    let auto_approve_adjustments =
        clap::value_t!(matches.value_of(AUTO_APPROVE_ADJUSTMENTS_ARG), bool)
            .expect("CLAP matches should have valid AUTO_APPROVE_ADJUSTMENTS_ARG");
    let ynab_budget_id = matches
        .value_of(YNAB_BUDGET_ID_ARG)
        .expect("CLAP matches should have YNAB_BUDGET_ID_ARG");
    let start_date_arg = matches
        .value_of(START_DATE_ARG)
        .map(parse_iso_date)
        .transpose()
        .expect("CLAP matches should have valid START_DATE_ARG");
    let ynab_client = YnabBudgetClient::new(
        matches
            .value_of(YNAB_ACCESS_TOKEN_ARG)
            .expect("CLAP matches should have YNAB_ACCESS_TOKEN_ARG")
            .to_string(),
        ynab_budget_id,
    );
    let currency_converter_client = CurrencyConverterClient::new(
        matches
            .value_of(CURRENCY_CONVERTER_API_KEY_ARG)
            .expect("CLAP matches should have CURRENCY_CONVERTER_API_KEY_ARG"),
        matches
            .value_of(CURRENCY_CONVERTER_API_BASE_URL_ARG)
            .expect("CLAP matches should have CURRENCY_CONVERTER_API_BASE_URL_ARG"),
        matches
            .value_of(CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST_ARG)
            .expect("CLAP matches should have CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST_ARG")
            .parse()
            .expect("CLAP matches should have valid CURRENCY_CONVERTER_API_MAX_CURRENCY_PAIRS_PER_REQUEST_ARG"),
    );
    let database = Database::establish_connection(
        matches
            .value_of(DATABASE_FILE_ARG)
            .expect("CLAP matches should have DATABASE_FILE_ARG"),
        dry_run,
    )?;
    ForeignTransactionsProcessor::run(
        &database,
        &ynab_client,
        &currency_converter_client,
        start_date_arg,
        dry_run,
        auto_approve_transactions,
        auto_approve_adjustments,
    )
}

fn default_env<V: AsRef<OsStr>>(var_name: &str, default_value: V) {
    if let Err(env::VarError::NotPresent) = env::var(var_name) {
        env::set_var(var_name, default_value);
    }
}
