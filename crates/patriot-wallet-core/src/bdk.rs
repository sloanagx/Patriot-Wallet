use bdk_esplora::{EsploraExt, esplora_client};
use bdk_wallet::{KeychainKind, Wallet};
use bitcoin::Network as BitcoinNetwork;

use crate::{BitcoinAccountXpub, Network, WalletError};

/// A BDK descriptor wallet containing public account data only.
///
/// It cannot sign because no private key is supplied to BDK.
pub struct BdkBitcoinWatchOnlyWallet {
    wallet: Wallet,
    network: Network,
}

/// A user-configured Esplora endpoint for one Bitcoin network.
///
/// Synchronizing with an endpoint reveals derived addresses to its operator.
pub struct BitcoinEsploraEndpoint {
    network: Network,
    base_url: String,
}

impl BitcoinEsploraEndpoint {
    pub fn new(network: Network, base_url: impl Into<String>) -> Result<Self, WalletError> {
        let base_url = base_url.into();

        if base_url.trim() != base_url || !base_url.starts_with("https://") {
            return Err(WalletError::InvalidEsploraEndpoint);
        }

        Ok(Self { network, base_url })
    }
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

        Ok(Self {
            wallet,
            network: account.network(),
        })
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

    /// Performs a read-only full scan through the configured Esplora endpoint.
    ///
    /// Sync history is kept only in memory at this stage; it will be persisted
    /// alongside BDK wallet state before transaction construction is enabled.
    pub fn full_scan(
        &mut self,
        endpoint: &BitcoinEsploraEndpoint,
        stop_gap: usize,
    ) -> Result<(), WalletError> {
        if endpoint.network != self.network {
            return Err(WalletError::EsploraNetworkMismatch);
        }

        if stop_gap == 0 {
            return Err(WalletError::InvalidSyncConfiguration);
        }

        let client = esplora_client::Builder::new(&endpoint.base_url).build_blocking();
        let update = client
            .full_scan(self.wallet.start_full_scan(), stop_gap, 5)
            .map_err(|_| WalletError::EsploraSyncFailed)?;

        self.wallet
            .apply_update(update)
            .map_err(|_| WalletError::EsploraSyncFailed)?;

        Ok(())
    }

    /// Total confirmed and unconfirmed balance in satoshis from the latest scan.
    pub fn balance_sats(&self) -> u64 {
        self.wallet.balance().total().to_sat()
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

    #[test]
    fn endpoint_requires_https_and_has_an_explicit_network() {
        assert!(BitcoinEsploraEndpoint::new(Network::Testnet, "https://example.test/api").is_ok());
        assert!(matches!(
            BitcoinEsploraEndpoint::new(Network::Testnet, "http://example.test/api"),
            Err(WalletError::InvalidEsploraEndpoint)
        ));
    }

    #[test]
    fn sync_rejects_an_endpoint_for_the_wrong_network_before_connecting() {
        let account = testnet_account();
        let mut bdk_wallet = BdkBitcoinWatchOnlyWallet::from_bip84_account(&account)
            .expect("BDK wallet should create");
        let endpoint = BitcoinEsploraEndpoint::new(Network::Mainnet, "https://example.test/api")
            .expect("endpoint should be valid");

        let result = bdk_wallet.full_scan(&endpoint, 20);

        assert!(matches!(result, Err(WalletError::EsploraNetworkMismatch)));
    }
}
