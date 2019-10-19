CREATE TABLE budgets (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  ynab_budget_id TEXT NOT NULL,
  start_date INTEGER NOT NULL,
  ynab_server_knowledge BIGINT NULL,
  last_run_date INTEGER NULL,
  UNIQUE(ynab_budget_id)
);
