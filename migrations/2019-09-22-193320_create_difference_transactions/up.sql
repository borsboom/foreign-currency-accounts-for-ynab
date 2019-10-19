CREATE TABLE difference_transactions (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  budget_id INT NOT NULL,
  foreign_ynab_transaction_id TEXT NOT NULL,
  difference_ynab_transaction_id TEXT NOT NULL,
  difference_amount_milliunits BIGINT NOT NULL,
  difference_currency_code TEXT NOT NULL,
  difference_is_tracking INTEGER NOT NULL,
  transfer_currency_code TEXT,
  transfer_is_tracking INTEGER,
  UNIQUE(budget_id, foreign_ynab_transaction_id),
  UNIQUE(budget_id, difference_ynab_transaction_id),
  FOREIGN KEY(budget_id) REFERENCES budgets(id)
);
