//! `Money<C>`, `Price`, and `Quantity` newtypes over `rust_decimal::Decimal`.
//!
//! Money arithmetic is only defined within the same currency — adding
//! `Money<Usdt>` to `Money<Btc>` is a compile-time error.
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::asset::Currency;
use crate::error::{PriceError, QtyError};

/// A monetary amount denominated in currency `C`.
/// Backed by `rust_decimal::Decimal`; no `f64` ever touches this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money<C: Currency> {
    amount: Decimal,
    _c: PhantomData<C>,
}

impl<C: Currency> Money<C> {
    /// Construct from a `Decimal` amount.
    #[must_use]
    pub fn from_decimal(amount: Decimal) -> Self {
        Self {
            amount,
            _c: PhantomData,
        }
    }

    /// Zero amount of this currency.
    #[must_use]
    pub fn zero() -> Self {
        Self::from_decimal(Decimal::ZERO)
    }

    /// Returns the inner `Decimal`.
    #[must_use]
    pub fn amount(&self) -> Decimal {
        self.amount
    }

    /// Absolute value.
    #[must_use]
    pub fn abs(&self) -> Self {
        Self::from_decimal(self.amount.abs())
    }

    /// Is the amount negative?
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.amount.is_sign_negative()
    }

    /// Is the amount zero?
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.amount.is_zero()
    }
}

impl<C: Currency> Add for Money<C> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::from_decimal(self.amount + rhs.amount)
    }
}

impl<C: Currency> AddAssign for Money<C> {
    fn add_assign(&mut self, rhs: Self) {
        self.amount += rhs.amount;
    }
}

impl<C: Currency> Sub for Money<C> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::from_decimal(self.amount - rhs.amount)
    }
}

impl<C: Currency> SubAssign for Money<C> {
    fn sub_assign(&mut self, rhs: Self) {
        self.amount -= rhs.amount;
    }
}

impl<C: Currency> Neg for Money<C> {
    type Output = Self;
    fn neg(self) -> Self {
        Self::from_decimal(-self.amount)
    }
}

impl<C: Currency> Mul<Decimal> for Money<C> {
    type Output = Self;
    fn mul(self, rhs: Decimal) -> Self {
        Self::from_decimal(self.amount * rhs)
    }
}

impl<C: Currency> std::fmt::Display for Money<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.amount, C::CODE)
    }
}

// Serde — serialize as the decimal string, deserialize same.
impl<C: Currency> Serialize for Money<C> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&self.amount, s)
    }
}

impl<'de, C: Currency> Deserialize<'de> for Money<C> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let amount = <Decimal as serde::Deserialize>::deserialize(d)?;
        Ok(Self::from_decimal(amount))
    }
}

// ── Price ────────────────────────────────────────────────────────────────────

/// A strictly-positive price. Backed by `Decimal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Price(Decimal);

impl Price {
    /// Construct a `Price`, returning an error if `d <= 0`.
    ///
    /// # Errors
    ///
    /// Returns [`PriceError::NonPositive`] if `d <= 0`.
    pub fn new(d: Decimal) -> Result<Self, PriceError> {
        if d <= Decimal::ZERO {
            return Err(PriceError::NonPositive(d));
        }
        Ok(Self(d))
    }

    /// Returns the inner `Decimal`.
    #[must_use]
    pub fn get(&self) -> Decimal {
        self.0
    }
}

impl std::fmt::Display for Price {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Quantity ─────────────────────────────────────────────────────────────────

/// A non-negative quantity. Signedness is carried by `Side`, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Quantity(Decimal);

impl Quantity {
    /// Construct a `Quantity`, returning an error if `d < 0`.
    ///
    /// # Errors
    ///
    /// Returns [`QtyError::Negative`] if `d < 0`.
    pub fn new(d: Decimal) -> Result<Self, QtyError> {
        if d < Decimal::ZERO {
            return Err(QtyError::Negative(d));
        }
        Ok(Self(d))
    }

    /// Zero quantity.
    #[must_use]
    pub fn zero() -> Self {
        Self(Decimal::ZERO)
    }

    /// Returns the inner `Decimal`.
    #[must_use]
    pub fn get(&self) -> Decimal {
        self.0
    }

    /// Is zero?
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl std::fmt::Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add for Quantity {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Quantity {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        // May temporarily produce negative during subtraction — caller validates
        Self(self.0 - rhs.0)
    }
}
