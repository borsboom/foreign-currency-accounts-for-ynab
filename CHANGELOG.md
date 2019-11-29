# Change log


## 0.1.7

Changes since 0.1.6:
- Support using non-free Currency Converter API by specifying base URL and
  maximum currency pairs per request
  ([#1](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/1)).

## 0.1.6

Changes since 0.1.5:
- Only use start date when downloading initial run transactions (fixes
  [#7](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/7)).
- Round difference transactions to budget's decimal digits (fixes
  [#8](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/8)).


## 0.1.5

Changes since 0.1.4:
- Include tracking account indicator in adjustments payee name.
- If a split transaction's payee is blank, use the parent's payee for the
  difference transaction (fixes
  [#2](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/2)).


## 0.1.4

Changes since 0.1.3:
- Don't try to update a difference transaction that was manually deleted.


## 0.1.3

Changes since 0.1.2:
- Fix unapproved matched import transaction handling
- Prevent ever modifying "deleted" (zeroed) difference transactions


## 0.1.2

Changes since 0.1.1:
- Fix extra group separator in currency formatting for negative numbers when
  minus not before symbol.
- Save last-knowledge-of-server if dry-run doesn't detect any changes need to
  be made, to avoid unnecessarily re-retrieving accounts.


## 0.1.1

Changes since 0.1.0:
- Load configuration file from system configuration folder.


## 0.1.0

Initial release!
