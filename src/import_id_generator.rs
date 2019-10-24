use std::cell::RefCell;

use crate::constants::*;
use crate::types::*;

#[derive(Debug)]
pub struct ImportIdGenerator {
    prefix: String,
    next_number: RefCell<i32>,
}

impl ImportIdGenerator {
    pub fn new() -> ImportIdGenerator {
        ImportIdGenerator {
            prefix: format!(
                "{}:{}",
                IMPORT_ID_PREFIX,
                chrono::Utc::now().format("%Y%m%d:%H%M%S%3f")
            ),
            next_number: RefCell::new(0),
        }
    }

    pub fn next_import_id(&self) -> YnabImportId {
        let mut next_number = self.next_number.borrow_mut();
        let result = format!("{}:{}", self.prefix, next_number);
        *next_number += 1;
        YnabImportId::new(result)
    }
}
