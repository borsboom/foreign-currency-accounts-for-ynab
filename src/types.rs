use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};
use std::borrow::Cow;
use std::fmt;
use std::ops;

use crate::errors::*;

pub use rust_decimal::prelude::Zero;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CurrencyCode([u8; 3]);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Milliunits(Decimal);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ExchangeRate(Decimal);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct YnabTransactionId(pub String);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct YnabImportId(pub String);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct YnabAccountId(pub String);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DifferenceKey {
    pub currency: CurrencyCode,
    pub is_tracking: bool,
}

impl CurrencyCode {
    pub fn from_str(code: &str) -> Result<CurrencyCode> {
        match code.as_bytes() {
            [a, b, c] => Ok(CurrencyCode([*a, *b, *c])),
            _ => bail!("Invalid currency code: {}", code),
        }
    }

    pub fn to_str(&self) -> Cow<str> {
        // Safe to use 'from_utf8_lossy', since we know our bytes
        // originally came from a 'String'.
        String::from_utf8_lossy(&self.0)
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

impl Milliunits {
    const SCALE: u32 = 3;

    pub fn from_scaled_i64(value: i64) -> Milliunits {
        Milliunits(Decimal::new(value, Self::SCALE))
    }

    pub fn to_scaled_i64(self) -> i64 {
        assert!(
            self.0.scale() == Self::SCALE,
            "Milliunits Decimal scale should be {}, but is {}",
            Self::SCALE,
            self.0.scale()
        );
        let mut result = self.0;
        result
            .set_scale(0)
            .expect("Milliunits Decimal scale should be settable to 0");
        result
            .to_i64()
            .expect("Milliunits Decimal should be convertible to i64")
    }

    pub fn from_decimal(value: Decimal) -> Milliunits {
        let scale_difference = Self::SCALE as i32 - value.scale() as i32;
        if scale_difference.is_zero() {
            return Milliunits(value);
        }
        let mut result = if scale_difference > 0 {
            value * Decimal::new(10i64.pow(scale_difference as u32), 0)
        } else {
            value / Decimal::new(10i64.pow((-scale_difference) as u32), 0)
        };
        result
            .set_scale(Self::SCALE)
            .unwrap_or_else(|_| panic!("Milliunits scale should be settable to {}", Self::SCALE));
        Milliunits(result)
    }

    pub fn to_decimal(self) -> Decimal {
        self.0
    }

    pub fn convert_currency(self, exchange_rate: ExchangeRate) -> Milliunits {
        Milliunits::from_decimal(
            (self.0 * exchange_rate.0)
                .round_dp_with_strategy(Self::SCALE, RoundingStrategy::BankersRounding),
        )
    }

    pub fn abs(&self) -> Milliunits {
        let result = Milliunits(self.0.abs());
        assert_eq!(result.0.scale(), Self::SCALE);
        result
    }

    pub fn smallest_unit(currency_decimal_digits: u32) -> Milliunits {
        assert!(
            currency_decimal_digits <= Self::SCALE,
            "Decimal points may not be greater than scale {}",
            Self::SCALE
        );
        Milliunits::from_decimal(
            Decimal::new(1, 0) / Decimal::new(10i64.pow(currency_decimal_digits), 0),
        )
    }
}

impl ops::Add for Milliunits {
    type Output = Milliunits;
    fn add(self, other: Milliunits) -> Milliunits {
        let result = Milliunits(self.0 + other.0);
        assert_eq!(result.0.scale(), Self::SCALE);
        result
    }
}

impl ops::AddAssign for Milliunits {
    fn add_assign(&mut self, other: Milliunits) {
        self.0 += other.0;
        assert_eq!(self.0.scale(), Self::SCALE);
    }
}

impl ops::Sub for Milliunits {
    type Output = Milliunits;
    fn sub(self, other: Milliunits) -> Milliunits {
        let result = Milliunits(self.0 - other.0);
        assert_eq!(result.0.scale(), Self::SCALE);
        result
    }
}

impl ops::SubAssign for Milliunits {
    fn sub_assign(&mut self, other: Milliunits) {
        self.0 -= other.0;
        assert_eq!(self.0.scale(), Self::SCALE);
    }
}

impl ops::Neg for Milliunits {
    type Output = Milliunits;
    fn neg(self) -> Milliunits {
        let result = Milliunits(self.0.neg());
        assert_eq!(result.0.scale(), Self::SCALE);
        result
    }
}

impl Zero for Milliunits {
    fn zero() -> Milliunits {
        Milliunits::from_scaled_i64(0)
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl ExchangeRate {
    const SCALE: u32 = 6;

    pub fn from_scaled_i64(value: i64) -> ExchangeRate {
        ExchangeRate(Decimal::new(value, Self::SCALE))
    }

    pub fn to_scaled_i64(self) -> i64 {
        assert!(
            self.0.scale() == Self::SCALE,
            "ExchangeRate Decimal scale should be {}, but is {}",
            Self::SCALE,
            self.0.scale()
        );
        let mut result = self.0;
        result
            .set_scale(0)
            .expect("Milliunits Decimal scale should be settable to 0");
        result
            .to_i64()
            .expect("Milliunits Decimal should be convertible to i64")
    }

    pub fn to_decimal(self) -> Decimal {
        self.0
    }

    pub fn from_f64(rate: f64) -> ExchangeRate {
        ExchangeRate(Decimal::new(
            (rate * 10.0f64.powi(Self::SCALE as i32)).round() as i64,
            Self::SCALE,
        ))
    }
}

impl fmt::Display for YnabTransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for YnabImportId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl DifferenceKey {
    pub fn new(currency: CurrencyCode, is_tracking: bool) -> DifferenceKey {
        DifferenceKey {
            currency,
            is_tracking,
        }
    }
}

impl fmt::Display for YnabAccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for DifferenceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} account for {}",
            if self.is_tracking {
                "tracking"
            } else {
                "budget"
            },
            self.currency
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_milliunits_from_to_scaled_i64() {
        assert_eq!(Milliunits::from_scaled_i64(12_345).to_scaled_i64(), 12_345);
    }

    #[test]
    fn test_milliunits_convert_currency() {
        assert_eq!(
            Milliunits::from_scaled_i64(12_345)
                .convert_currency(ExchangeRate::from_scaled_i64(1_234_567)),
            Milliunits::from_scaled_i64(15_241)
        )
    }

    #[test]
    fn test_milliunits_smallest_unit() {
        assert_eq!(
            Milliunits::smallest_unit(0),
            Milliunits::from_scaled_i64(1000)
        );
        assert_eq!(
            Milliunits::smallest_unit(1),
            Milliunits::from_scaled_i64(100)
        );
        assert_eq!(
            Milliunits::smallest_unit(2),
            Milliunits::from_scaled_i64(10)
        );
        assert_eq!(Milliunits::smallest_unit(3), Milliunits::from_scaled_i64(1));
    }

    #[test]
    fn test_exchange_rate_from_to_scaled_i64() {
        assert_eq!(
            ExchangeRate::from_scaled_i64(12_345_678).to_scaled_i64(),
            12_345_678
        );
    }

    #[test]
    fn test_exchange_rate_from_f64() {
        assert_eq!(
            ExchangeRate::from_f64(12.345_678),
            ExchangeRate::from_scaled_i64(12_345_678)
        );
        assert_eq!(
            ExchangeRate::from_f64(12.345_678_5),
            ExchangeRate::from_scaled_i64(12_345_679)
        );
    }
}
