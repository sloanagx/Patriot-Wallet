use serde::{Deserialize, Serialize};

use crate::{
    BdkBitcoinWatchOnlyWallet, BitcoinAccountXpub, BitcoinReceiveAddress, Network, WalletError,
};

const METADATA_FORMAT_VERSION: u8 = 1;

/// Versioned public metadata required to restore receive-address allocation.
///
/// This intentionally contains no recovery phrase, seed, or private key.
#[derive(Serialize, Deserialize)]
pub struct BitcoinReceiveAccountMetadata {
    pub format_version: u8,
    pub account_xpub: String,
    pub network: Network,
    pub next_receive_address_index: u32,
}

/// Allocates unique Bitcoin BIP-84 receive addresses during one wallet session.
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

    pub fn watch_only_wallet(&self) -> Result<BdkBitcoinWatchOnlyWallet, WalletError> {
        BdkBitcoinWatchOnlyWallet::from_bip84_account(&self.account)
    }

    pub fn watch_only_wallet_with_state(
        &self,
        path: impl Into<std::path::PathBuf>,
    ) -> Result<BdkBitcoinWatchOnlyWallet, WalletError> {
        match BdkBitcoinWatchOnlyWallet::load_state(&self.account, path)? {
            Some(wallet) => Ok(wallet),
            None => self.watch_only_wallet(),
        }
    }

    pub fn metadata(&self) -> BitcoinReceiveAccountMetadata {
        BitcoinReceiveAccountMetadata {
            format_version: METADATA_FORMAT_VERSION,
            account_xpub: self.account.encoded(),
            network: self.account.network(),
            next_receive_address_index: self.next_receive_index,
        }
    }

    pub fn restore(metadata: BitcoinReceiveAccountMetadata) -> Result<Self, WalletError> {
        if metadata.format_version != METADATA_FORMAT_VERSION {
            return Err(WalletError::UnsupportedMetadataVersion);
        }

        let account = BitcoinAccountXpub::import(&metadata.account_xpub, metadata.network)?;

        Ok(Self {
            account,
            next_receive_index: metadata.next_receive_address_index,
        })
    }

    /// Reserves and returns the next external-chain receive address.
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
    use crate::{RecoveryPhrase, derive_bitcoin_bip84_account};

    fn testnet_account() -> BitcoinReceiveAccount {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");
        let seed = phrase.derive_seed();
        let account = derive_bitcoin_bip84_account(&seed, Network::Testnet)
            .expect("testnet account derivation should succeed");

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

    #[test]
    fn metadata_round_trip_preserves_the_next_receive_index() {
        let mut account = testnet_account();

        account
            .issue_next_receive_address()
            .expect("first address should derive");
        account
            .issue_next_receive_address()
            .expect("second address should derive");

        let json = serde_json::to_string(&account.metadata()).expect("metadata should serialize");
        let metadata = serde_json::from_str(&json).expect("metadata should deserialize");

        let mut restored =
            BitcoinReceiveAccount::restore(metadata).expect("metadata should restore");

        let next = restored
            .issue_next_receive_address()
            .expect("third address should derive");

        assert_eq!(next.index(), 2);
    }

    #[test]
    fn unsupported_metadata_version_is_rejected() {
        let account = testnet_account();
        let mut metadata = account.metadata();
        metadata.format_version = 2;

        let result = BitcoinReceiveAccount::restore(metadata);

        assert!(matches!(
            result,
            Err(WalletError::UnsupportedMetadataVersion)
        ));
    }

    #[test]
    fn missing_state_creates_a_wallet_that_can_be_saved_and_reopened() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.json");
        let account = testnet_account();

        let mut wallet = account.watch_only_wallet_with_state(&path).unwrap();
        let expected_address = wallet.receive_address_at(0);

        wallet.save_state(&path).unwrap();

        let reopened = account.watch_only_wallet_with_state(&path).unwrap();
        assert_eq!(reopened.receive_address_at(0), expected_address);
    }

    #[test]
    fn corrupt_state_is_not_silently_replaced_with_a_fresh_wallet() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.json");
        let account = testnet_account();

        std::fs::write(&path, b"damaged state").unwrap();

        assert!(matches!(
            account.watch_only_wallet_with_state(&path),
            Err(WalletError::InvalidStoredWalletState)
        ));

        assert_eq!(std::fs::read(&path).unwrap(), b"damaged state");
    }
}
