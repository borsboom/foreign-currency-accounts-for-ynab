CREATE TABLE exchange_rates (
  id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
  date INTEGER NOT NULL,
  from_currency_code TEXT NOT NULL,
  to_currency_code TEXT NOT NULL,
  exchange_rate BIGINT NOT NULL,
  UNIQUE(date, from_currency_code, to_currency_code)
);
