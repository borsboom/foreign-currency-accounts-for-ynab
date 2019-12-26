ALTER TABLE difference_transactions RENAME TO old_difference_transactions_20191226;

CREATE TABLE difference_transactions (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  budget_id INT NOT NULL,
  foreign_ynab_transaction_id TEXT NOT NULL,
  difference_ynab_transaction_id TEXT NOT NULL,
  difference_amount_milliunits BIGINT NOT NULL,
  difference_currency_code TEXT NOT NULL,
  difference_account_class TEXT NOT NULL,
  transfer_currency_code TEXT,
  transfer_account_class TEXT,
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
  'D',
  transfer_currency_code,
  NULL
FROM old_difference_transactions_20191226;

UPDATE difference_transactions
SET difference_account_class = 'T'
WHERE id IN (SELECT id FROM old_difference_transactions_20191226 WHERE difference_is_tracking = 1);

UPDATE difference_transactions
SET difference_account_class = 'C'
WHERE id IN (SELECT id FROM old_difference_transactions_20191226 WHERE difference_is_credit = 1);

UPDATE difference_transactions
SET transfer_account_class = 'D'
WHERE id IN (SELECT id FROM old_difference_transactions_20191226 WHERE transfer_is_tracking = 0 OR transfer_is_credit = 0);

UPDATE difference_transactions
SET transfer_account_class = 'T'
WHERE id IN (SELECT id FROM old_difference_transactions_20191226 WHERE transfer_is_tracking = 1);

UPDATE difference_transactions
SET transfer_account_class = 'C'
WHERE id IN (SELECT id FROM old_difference_transactions_20191226 WHERE transfer_is_credit = 1);

DROP TABLE old_difference_transactions_20191226;
