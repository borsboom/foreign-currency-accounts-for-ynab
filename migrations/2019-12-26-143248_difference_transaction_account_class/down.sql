ALTER TABLE difference_transactions RENAME TO new_difference_transactions_20191226;

CREATE TABLE difference_transactions (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  budget_id INT NOT NULL,
  foreign_ynab_transaction_id TEXT NOT NULL,
  difference_ynab_transaction_id TEXT NOT NULL,
  difference_amount_milliunits BIGINT NOT NULL,
  difference_currency_code TEXT NOT NULL,
  difference_is_credit INTEGER NOT NULL,
  difference_is_tracking INTEGER NOT NULL,
  transfer_currency_code TEXT,
  transfer_is_credit INTEGER,
  transfer_is_tracking INTEGER,
  UNIQUE(budget_id, foreign_ynab_transaction_id),
  UNIQUE(budget_id, difference_ynab_transaction_id),
  FOREIGN KEY(budget_id) REFERENCES budgets(id)
);

INSERT INTO difference_transactions
SELECT
  id,
  budget_id,
  foreign_ynab_transaction_id,
  difference_ynab_transaction_id,
  difference_amount_milliunits,
  difference_currency_code,
  0,
  0,
  transfer_currency_code,
  NULL,
  NULL
FROM new_difference_transactions_20191226;

UPDATE difference_transactions
SET difference_is_tracking = 1
WHERE id IN (SELECT id FROM new_difference_transactions_20191226 WHERE difference_account_class = 'T');

UPDATE difference_transactions
SET difference_is_credit = 1
WHERE id IN (SELECT id FROM new_difference_transactions_20191226 WHERE difference_account_class = 'C');

UPDATE difference_transactions
SET transfer_is_tracking = 1
WHERE id IN (SELECT id FROM new_difference_transactions_20191226 WHERE transfer_account_class = 'T');

UPDATE difference_transactions
SET transfer_is_credit = 1
WHERE id IN (SELECT id FROM new_difference_transactions_20191226 WHERE transfer_account_class = 'C');

DROP TABLE new_difference_transactions_20191226;
