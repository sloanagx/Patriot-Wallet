use bitcoin::Amount;

/// Bitcoin amount represented in satoshis.
///
/// this type intentionally has no floating-point constructors.
///

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BitcoinAmount(Amount);

impl BitcoinAmount {
    pub const fn from_sats(sats: u64) -> Self {
        Self(Amount::from_sat(sats))
    }

    pub fn sats(self) -> u64 {
        self.0.to_sat()
    }

    pub fn checked_add(self, other: Self) -> Result<Self, WalletError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(WalletError::AmountOverflow)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, WalletError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or(WalletError::InsufficientFunds)
    }
}

use crate::WalletError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_satoshi_precision() {
        let amount = BitcoinAmount::from_sats(123_456_789);

        assert_eq!(amount.sats(), 123_456_789);
    }

    #[test]
    fn adds_amounts_without_floating_point_math() {
        let total = BitcoinAmount::from_sats(50_000)
            .checked_add(BitcoinAmount::from_sats(25_000))
            .expect("addition should not overflow");

        assert_eq!(total.sats(), 75_000);
    }

    #[test]
    fn rejects_subtraction_larger_than_available_balance() {
        let result = BitcoinAmount::from_sats(10).checked_sub(BitcoinAmount::from_sats(11));

        assert_eq!(result, Err(WalletError::InsufficientFunds));
    }

    #[test]
    fn rejects_overflow() {
        let result = BitcoinAmount::from_sats(u64::MAX).checked_add(BitcoinAmount::from_sats(1));

        assert_eq!(result, Err(WalletError::AmountOverflow));
    }
}
