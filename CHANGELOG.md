# Change log


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
