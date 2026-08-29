use crate::{BitcoinAccountXpub, BitcoinReceiveAddress, WalletError};

/// Allocates unique Bitcoin BIP-84 receive addresses during one wallet session.
///
/// This state is not persistent yet. It must be saved atomically before any
/// address is presented to a user in a production wallet.
///

pub struct BitcoinReceiveAccount {
    account: BitcoinAccountXpub,
    next_receive_index: u32,
}

impl BitcoinReceiveAccount {
    pub fn new(account: BitcoinAccountXpub) -> Self {
        Self {
            account,
            next_receive_index: 0,
        }
    }

    pub const fn next_receive_index(&self) -> u32 {
        self.next_receive_index
    }

    /// Reserves and returns the next external chain receive address.
    ///
    /// The index advances only after successful address derivation. If the derivation fails, the index is not advanced.
    pub fn issue_next_receive_address(&mut self) -> Result<BitcoinReceiveAddress, WalletError> {
        let index = self.next_receive_index;

        if index >= (1 << 31) {
            return Err(WalletError::ReceiveAddressIndexExhausted);
        }

        let address = self.account.derive_receive_address(index)?;

        self.next_receive_index = index
            .checked_add(1)
            .ok_or(WalletError::ReceiveAddressIndexExhausted)?;

        Ok(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Network, RecoveryPhrase, derive_bitcoin_bip84_account};

    fn testnet_account() -> BitcoinReceiveAccount {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");
        let seed = phrase.derive_seed();
        let account = derive_bitcoin_bip84_account(&seed, Network::Testnet)
            .expect("Testnet account derivation should succeed");
        BitcoinReceiveAccount::new(account)
    }

    #[test]
    fn first_reserved_address_uses_index_zero() {
        let mut account = testnet_account();

        let address = account
            .issue_next_receive_address()
            .expect("first address should derive");

        assert_eq!(address.index(), 0);
        assert_eq!(account.next_receive_index(), 1);
    }

    #[test]
    fn each_reserved_address_advances_to_a_new_index() {
        let mut account = testnet_account();

        let first = account
            .issue_next_receive_address()
            .expect("first address should derive");
        let second = account
            .issue_next_receive_address()
            .expect("second address should derive");

        assert_eq!(second.index(), 1);
        assert_ne!(first.encoded(), second.encoded());
        assert_eq!(account.next_receive_index(), 2);
    }
}
