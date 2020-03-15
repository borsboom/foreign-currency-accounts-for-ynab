# Change log


## 0.1.11

Changes since 0.1.10:
- Round adjustment transactions to currency format's decimal digits, to avoid
  risk of "cycles" when there are sub-cent account balances present (fixes
  [#23](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/23)).
- Build ARMv7 Linux binaries, for Raspberry Pi and similar (fixes
  [#24](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/24)).

## 0.1.10

**BREAKING CHANGE:** If you have foreign currency **credit** accounts, you must
now create a separate difference account for those (so that YNAB's special
credit handling applies to the converted amounts too).  Follow the same
instructions as for a regular (debit) account, but choose account type **Credit
Card** instead.  You should add additional text to the nickname before or after
the "tag" to differentiate it from the debit difference account, just make sure
the extra text is outside the angle brackets (for example, `Credit <EUR
DIFFERENCE>`).

Changes since 0.1.9:
- Use separate difference accounts for credit accounts (fixes
  [#13](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/13)).
- Set more consistent difference transaction memo prefixes.  Now they all start
  with `<CONVERT: …>`.


## 0.1.9

Changes since 0.1.8:
- Skip transactions in closed and deleted accounts instead of failing (fixes
  [#19](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/19)).
- Treat empty string flag_color from YNAB API same as null (fixes
  [#17](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/17)).
  This can happen if you use the Toolkit for YNAB extension to clear flags.
- Clear flag in difference transaction when source transaction flag cleared.


## 0.1.8

Changes since 0.1.7
- Fix behavior when changing an un-split transaction to a split, and vice-versa
  (fixes
  [#12](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/12)).
- Default start date to 30 days ago (fixes
  [#7](https://github.com/borsboom/foreign-currency-accounts-for-ynab/issues/7)).
- Always use start date when downloading latest transaction (reverts change
  from 0.1.6).

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
