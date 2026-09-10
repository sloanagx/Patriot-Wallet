use bdk_wallet::{KeychainKind, Wallet};
use bitcoin::Network as BitcoinNetwork;

use crate::{BitcoinAccountXpub, Network, WalletError};

/// A BDK descriptor wallet containing public account data only.
///
/// It cannot sign because no private key is supplied to BDK.
pub struct BdkBitcoinWatchOnlyWallet {
    wallet: Wallet,
}

impl BdkBitcoinWatchOnlyWallet {
    pub fn from_bip84_account(account: &BitcoinAccountXpub) -> Result<Self, WalletError> {
        let xpub = account.encoded();
        let receive_descriptor = format!("wpkh({xpub}/0/*)");
        let change_descriptor = format!("wpkh({xpub}/1/*)");

        let bitcoin_network = match account.network() {
            Network::Mainnet => BitcoinNetwork::Bitcoin,
            Network::Testnet => BitcoinNetwork::Testnet,
        };

        let wallet = Wallet::create(receive_descriptor, change_descriptor)
            .network(bitcoin_network)
            .create_wallet_no_persist()
            .map_err(|_| WalletError::BdkWalletCreationFailed)?;

        Ok(Self { wallet })
    }

    /// Returns a deterministic receive address without marking it as used.
    pub fn receive_address_at(&self, index: u32) -> String {
        self.wallet
            .peek_address(KeychainKind::External, index)
            .address
            .to_string()
    }

    /// Returns a deterministic change address without marking it as used.
    pub fn change_address_at(&self, index: u32) -> String {
        self.wallet
            .peek_address(KeychainKind::Internal, index)
            .address
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RecoveryPhrase, derive_bitcoin_bip84_account};

    fn testnet_account() -> BitcoinAccountXpub {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");
        let seed = phrase.derive_seed();

        derive_bitcoin_bip84_account(&seed, Network::Testnet)
            .expect("testnet account derivation should succeed")
    }

    #[test]
    fn bdk_receive_descriptor_matches_existing_bip84_derivation() {
        let account = testnet_account();
        let expected = account
            .derive_receive_address(0)
            .expect("receive address should derive");

        let bdk_wallet = BdkBitcoinWatchOnlyWallet::from_bip84_account(&account)
            .expect("BDK wallet should create");

        assert_eq!(bdk_wallet.receive_address_at(0), expected.encoded());
    }

    #[test]
    fn bdk_derives_testnet_receive_and_change_addresses() {
        let account = testnet_account();
        let bdk_wallet = BdkBitcoinWatchOnlyWallet::from_bip84_account(&account)
            .expect("BDK wallet should create");

        let receive = bdk_wallet.receive_address_at(0);
        let change = bdk_wallet.change_address_at(0);

        assert!(receive.starts_with("tb1q"));
        assert!(change.starts_with("tb1q"));
        assert_ne!(receive, change);
    }
}
