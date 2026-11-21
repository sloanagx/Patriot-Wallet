use crate::bitcoin_state::{BitcoinWalletState, BitcoinWalletStateStore};
use crate::{BitcoinAccountXpub, Network, WalletError};
use bdk_esplora::{EsploraExt, esplora_client};
use bdk_wallet::{ChangeSet, KeychainKind, Wallet, chain::Merge};
use bitcoin::Network as BitcoinNetwork;

/// A BDK descriptor wallet containing public account data only.
///
/// It cannot sign because no private key is supplied to BDK.
pub struct BdkBitcoinWatchOnlyWallet {
    wallet: Wallet,
    network: Network,
    accumulated_state: ChangeSet,
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

        let mut wallet = Wallet::create(receive_descriptor, change_descriptor)
            .network(bitcoin_network)
            .create_wallet_no_persist()
            .map_err(|_| WalletError::BdkWalletCreationFailed)?;

        let accumulated_state = wallet
            .take_staged()
            .ok_or(WalletError::BdkWalletCreationFailed)?;

        Ok(Self {
            wallet,
            network: account.network(),
            accumulated_state,
        })
    }

    /// Captures the complete state while retaining it for subsequent saves.
    pub(crate) fn state_snapshot(&mut self) -> ChangeSet {
        if let Some(changes) = self.wallet.take_staged() {
            self.accumulated_state.merge(changes);
        }

        self.accumulated_state.clone()
    }

    /// Atomically saves the complete public watch-only state.
    ///
    /// If saving fails, the accumulated state remains available for retry.
    pub fn save_state(&mut self, path: impl Into<std::path::PathBuf>) -> Result<(), WalletError> {
        let state = BitcoinWalletState::new(self.state_snapshot());
        BitcoinWalletStateStore::new(path).save(&state)
    }

    /// Loads saved state only if it matches the expected account and network.
    ///
    /// A missing file returns None. Invalid or mismatched state returns an error.
    pub fn load_state(
        account: &BitcoinAccountXpub,
        path: impl Into<std::path::PathBuf>,
    ) -> Result<Option<Self>, WalletError> {
        let Some(accumulated_state) = BitcoinWalletStateStore::new(path).load()? else {
            return Ok(None);
        };

        let bitcoin_network = match account.network() {
            Network::Mainnet => BitcoinNetwork::Bitcoin,
            Network::Testnet => BitcoinNetwork::Testnet,
        };

        let xpub = account.encoded();
        let genesis_hash =
            bitcoin::blockdata::constants::genesis_block(bitcoin_network).block_hash();

        let wallet = Wallet::load()
            .descriptor(KeychainKind::External, Some(format!("wpkh({xpub}/0/*)")))
            .descriptor(KeychainKind::Internal, Some(format!("wpkh({xpub}/1/*)")))
            .check_network(bitcoin_network)
            .check_genesis_hash(genesis_hash)
            .load_wallet_no_persist(accumulated_state.clone())
            .map_err(|_| WalletError::BdkWalletLoadFailed)?
            .ok_or(WalletError::InvalidStoredWalletState)?;

        Ok(Some(Self {
            wallet,
            network: account.network(),
            accumulated_state,
        }))
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
    /// Updates remain in memory until the caller successfully calls save_state.
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

    #[test]
    fn snapshots_preserve_initial_state_and_accumulate_address_updates() {
        let account = testnet_account();
        let mut wallet =
            BdkBitcoinWatchOnlyWallet::from_bip84_account(&account).expect("wallet should create");

        wallet.wallet.reveal_next_address(KeychainKind::External);
        let first = wallet.state_snapshot();

        wallet.wallet.reveal_next_address(KeychainKind::External);
        let second = wallet.state_snapshot();

        assert!(second.descriptor.is_some());
        assert!(second.change_descriptor.is_some());
        assert_eq!(second.network, Some(BitcoinNetwork::Testnet));
        assert_eq!(second.local_chain, first.local_chain);

        assert_eq!(first.indexer.last_revealed.values().copied().max(), Some(0));
        assert_eq!(
            second.indexer.last_revealed.values().copied().max(),
            Some(1)
        );

        // Capturing again without changes must preserve the complete state.
        assert_eq!(wallet.state_snapshot(), second);
    }

    #[test]
    fn wallet_state_can_be_saved_multiple_times() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bitcoin-state.json");
        let account = testnet_account();
        let mut wallet = BdkBitcoinWatchOnlyWallet::from_bip84_account(&account).unwrap();

        wallet.save_state(&path).unwrap();

        wallet.wallet.reveal_next_address(KeychainKind::External);
        wallet.save_state(&path).unwrap();

        let stored = BitcoinWalletStateStore::new(&path)
            .load()
            .unwrap()
            .expect("saved state should exist");

        assert_eq!(stored, wallet.state_snapshot());
        assert!(stored.descriptor.is_some());
        assert_eq!(
            stored.indexer.last_revealed.values().copied().max(),
            Some(0)
        );
    }

    #[test]
    fn failed_save_can_be_retried_without_losing_state() {
        let directory = tempfile::tempdir().unwrap();
        let blocked_parent = directory.path().join("not-a-directory");
        std::fs::write(&blocked_parent, b"blocks directory creation").unwrap();

        let account = testnet_account();
        let mut wallet = BdkBitcoinWatchOnlyWallet::from_bip84_account(&account).unwrap();
        wallet.wallet.reveal_next_address(KeychainKind::External);

        let expected = wallet.state_snapshot();

        assert!(matches!(
            wallet.save_state(blocked_parent.join("state.json")),
            Err(WalletError::WalletStateStorageFailed)
        ));

        let valid_path = directory.path().join("state.json");
        wallet.save_state(&valid_path).unwrap();

        let stored = BitcoinWalletStateStore::new(valid_path)
            .load()
            .unwrap()
            .expect("retried save should exist");

        assert_eq!(stored, expected);
    }

    #[test]
    fn restored_wallet_continues_from_saved_address_index() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.json");
        let account = testnet_account();
        let mut original = BdkBitcoinWatchOnlyWallet::from_bip84_account(&account).unwrap();

        original.wallet.reveal_next_address(KeychainKind::External);
        original.wallet.reveal_next_address(KeychainKind::External);
        original.save_state(&path).unwrap();

        let mut restored = BdkBitcoinWatchOnlyWallet::load_state(&account, &path)
            .unwrap()
            .expect("saved wallet should load");

        assert_eq!(
            restored.receive_address_at(0),
            original.receive_address_at(0)
        );
        assert_eq!(restored.change_address_at(0), original.change_address_at(0));

        let next = restored.wallet.reveal_next_address(KeychainKind::External);
        assert_eq!(next.index, 2);

        restored.save_state(&path).unwrap();

        let mut reloaded = BdkBitcoinWatchOnlyWallet::load_state(&account, &path)
            .unwrap()
            .unwrap();

        assert_eq!(
            reloaded
                .wallet
                .reveal_next_address(KeychainKind::External)
                .index,
            3
        );
    }

    #[test]
    fn saved_state_from_another_account_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.json");
        let account = testnet_account();
        let other_account = testnet_account();
        let mut wallet = BdkBitcoinWatchOnlyWallet::from_bip84_account(&account).unwrap();

        wallet.save_state(&path).unwrap();

        assert!(matches!(
            BdkBitcoinWatchOnlyWallet::load_state(&other_account, &path),
            Err(WalletError::BdkWalletLoadFailed)
        ));
    }

    #[test]
    fn saved_state_with_wrong_network_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.json");
        let account = testnet_account();
        let mut wallet = BdkBitcoinWatchOnlyWallet::from_bip84_account(&account).unwrap();

        let mut state = wallet.state_snapshot();
        state.network = Some(BitcoinNetwork::Bitcoin);

        BitcoinWalletStateStore::new(&path)
            .save(&BitcoinWalletState::new(state))
            .unwrap();

        assert!(matches!(
            BdkBitcoinWatchOnlyWallet::load_state(&account, &path),
            Err(WalletError::BdkWalletLoadFailed)
        ));
    }

    #[test]
    fn received_transaction_and_balance_survive_reopening() {
        use bitcoin::{
            Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid, Witness,
            absolute, hashes::Hash, transaction,
        };

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.json");
        let account = testnet_account();
        let mut original = BdkBitcoinWatchOnlyWallet::from_bip84_account(&account).unwrap();

        let receive = original.wallet.reveal_next_address(KeychainKind::External);

        let payment = Transaction {
            version: transaction::Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_byte_array([1; 32]),
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::default(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(50_000),
                script_pubkey: receive.address.script_pubkey(),
            }],
        };

        let txid = payment.compute_txid();
        original
            .wallet
            .apply_unconfirmed_txs([(payment.clone(), 1_700_000_000)]);

        assert_eq!(original.balance_sats(), 50_000);
        original.save_state(&path).unwrap();
        drop(original);

        let mut restored = BdkBitcoinWatchOnlyWallet::load_state(&account, &path)
            .unwrap()
            .expect("saved wallet should load");

        assert_eq!(restored.balance_sats(), 50_000);

        let stored_payment = restored
            .wallet
            .tx_graph()
            .get_tx(txid)
            .expect("received transaction should survive reopening");

        assert_eq!(stored_payment.as_ref(), &payment);

        // Saving again without new updates must retain the transaction.
        restored.save_state(&path).unwrap();
        drop(restored);

        let reopened = BdkBitcoinWatchOnlyWallet::load_state(&account, &path)
            .unwrap()
            .unwrap();

        assert_eq!(reopened.balance_sats(), 50_000);
        assert!(reopened.wallet.tx_graph().get_tx(txid).is_some());
    }
}
