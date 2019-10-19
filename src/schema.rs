table! {
    budgets (id) {
        id -> Integer,
        ynab_budget_id -> Text,
        start_date -> Integer,
        ynab_server_knowledge -> Nullable<BigInt>,
        last_run_date -> Nullable<Integer>,
    }
}

table! {
    difference_transactions (id) {
        id -> Integer,
        budget_id -> Integer,
        foreign_ynab_transaction_id -> Text,
        difference_ynab_transaction_id -> Text,
        difference_amount_milliunits -> BigInt,
        difference_currency_code -> Text,
        difference_is_tracking -> Integer,
        transfer_currency_code -> Nullable<Text>,
        transfer_is_tracking -> Nullable<Integer>,
    }
}

table! {
    exchange_rates (id) {
        id -> Integer,
        date -> Integer,
        from_currency_code -> Text,
        to_currency_code -> Text,
        exchange_rate -> BigInt,
    }
}

joinable!(difference_transactions -> budgets (budget_id));

allow_tables_to_appear_in_same_query!(budgets, difference_transactions, exchange_rates,);
