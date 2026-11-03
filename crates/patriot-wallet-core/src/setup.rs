use std::path::PathBuf;

use crate::Network;
use crate::{
    BitcoinReceiveAccount, BitcoinReceiveAddress, DogecoinReceiveAccount, EthereumReceiveAccount,
    OsKeychainRecoveryVault, PersistentBitcoinReceiveAccount, PersistentDogecoinReceiveAccount,
    PersistentEthereumReceiveAccount, PersistentSolanaReceiveAccount, RecoveryPhrase,
    RecoveryPhraseVault, SolanaReceiveAccount, SolanaReceiveAddress, WalletError, WalletId,
    derive_bitcoin_bip84_account, derive_dogecoin_bip44_account, derive_ethereum_bip44_account,
    derive_solana_bip44_account_address,
};

/// A newly created or restored testnet wallet session.
///
/// Secret recovery data remains in the operating-system vault; this type holds
/// only public receive-account state.
pub struct TestnetWallet {
    wallet_id: WalletId,
    receive_account: PersistentBitcoinReceiveAccount,
}

impl TestnetWallet {
    pub fn wallet_id(&self) -> &WalletId {
        &self.wallet_id
    }

    pub const fn next_receive_index(&self) -> u32 {
        self.receive_account.next_receive_index()
    }

    pub fn issue_next_receive_address(&mut self) -> Result<BitcoinReceiveAddress, WalletError> {
        self.receive_account.issue_next_receive_address()
    }
}

/// Creates a testnet wallet and stores its recovery phrase in the OS vault.
///
/// The metadata file contains public watch-only account data only.
pub fn create_testnet_wallet(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
) -> Result<TestnetWallet, WalletError> {
    create_testnet_wallet_with_vault(wallet_id, metadata_path, &OsKeychainRecoveryVault)
}

/// Restores a testnet wallet after verifying its Keychain recovery phrase
/// matches the saved public metadata.
pub fn restore_testnet_wallet(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
) -> Result<TestnetWallet, WalletError> {
    restore_testnet_wallet_with_vault(wallet_id, metadata_path, &OsKeychainRecoveryVault)
}

/// Creates a testnet Ethereum receive account from a Keychain recovery phrase.
///
/// The resulting metadata file contains only the public Ethereum account key.
pub fn create_testnet_ethereum_receive_account(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
) -> Result<PersistentEthereumReceiveAccount, WalletError> {
    create_testnet_ethereum_receive_account_with_vault(
        wallet_id,
        metadata_path,
        &OsKeychainRecoveryVault,
    )
}

/// Verifies that the Keychain recovery phrase matches Ethereum public metadata.
pub fn restore_testnet_ethereum_receive_account(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
) -> Result<PersistentEthereumReceiveAccount, WalletError> {
    restore_testnet_ethereum_receive_account_with_vault(
        wallet_id,
        metadata_path,
        &OsKeychainRecoveryVault,
    )
}

fn restore_testnet_ethereum_receive_account_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<PersistentEthereumReceiveAccount, WalletError> {
    let receive_account = PersistentEthereumReceiveAccount::load(metadata_path)?;

    if receive_account.metadata().network != Network::Testnet {
        return Err(WalletError::WalletNetworkMismatch);
    }

    let phrase = vault.load(&wallet_id)?;
    let seed = phrase.derive_seed();
    let expected_account = derive_ethereum_bip44_account(&seed)?;

    if receive_account.metadata().account_xpub != expected_account.encoded() {
        return Err(WalletError::RecoveryPhraseDoesNotMatchMetadata);
    }

    Ok(receive_account)
}

/// Creates a testnet Dogecoin receive account from a Keychain recovery phrase.
///
/// The resulting metadata file contains only the public Dogecoin account key.
pub fn create_testnet_dogecoin_receive_account(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
) -> Result<PersistentDogecoinReceiveAccount, WalletError> {
    create_testnet_dogecoin_receive_account_with_vault(
        wallet_id,
        metadata_path,
        &OsKeychainRecoveryVault,
    )
}

/// Verifies that the Keychain recovery phrase matches Dogecoin public metadata.
pub fn restore_testnet_dogecoin_receive_account(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
) -> Result<PersistentDogecoinReceiveAccount, WalletError> {
    restore_testnet_dogecoin_receive_account_with_vault(
        wallet_id,
        metadata_path,
        &OsKeychainRecoveryVault,
    )
}

fn restore_testnet_dogecoin_receive_account_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<PersistentDogecoinReceiveAccount, WalletError> {
    let receive_account = PersistentDogecoinReceiveAccount::load(metadata_path)?;

    if receive_account.metadata().network != Network::Testnet {
        return Err(WalletError::WalletNetworkMismatch);
    }

    let phrase = vault.load(&wallet_id)?;
    let seed = phrase.derive_seed();
    let expected_account = derive_dogecoin_bip44_account(&seed, Network::Testnet)?;

    if receive_account.metadata().account_xpub != expected_account.encoded() {
        return Err(WalletError::RecoveryPhraseDoesNotMatchMetadata);
    }

    Ok(receive_account)
}

fn create_testnet_dogecoin_receive_account_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<PersistentDogecoinReceiveAccount, WalletError> {
    let metadata_path = metadata_path.into();

    if metadata_path.exists() {
        return Err(WalletError::MetadataAlreadyExists);
    }

    let phrase = vault.load(&wallet_id)?;
    let seed = phrase.derive_seed();
    let account = derive_dogecoin_bip44_account(&seed, Network::Testnet)?;
    let receive_account = DogecoinReceiveAccount::new(account);

    PersistentDogecoinReceiveAccount::create(metadata_path, receive_account)
}

fn create_testnet_ethereum_receive_account_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<PersistentEthereumReceiveAccount, WalletError> {
    let metadata_path = metadata_path.into();

    if metadata_path.exists() {
        return Err(WalletError::MetadataAlreadyExists);
    }

    let phrase = vault.load(&wallet_id)?;
    let seed = phrase.derive_seed();
    let account = derive_ethereum_bip44_account(&seed)?;
    let receive_account = EthereumReceiveAccount::new(account);

    PersistentEthereumReceiveAccount::create(metadata_path, receive_account)
}

fn restore_testnet_wallet_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<TestnetWallet, WalletError> {
    let receive_account = PersistentBitcoinReceiveAccount::load(metadata_path)?;

    if receive_account.metadata().network != Network::Testnet {
        return Err(WalletError::WalletNetworkMismatch);
    }

    let phrase = vault.load(&wallet_id)?;
    let seed = phrase.derive_seed();
    let expected_account = derive_bitcoin_bip84_account(&seed, Network::Testnet)?;

    if receive_account.metadata().account_xpub != expected_account.encoded() {
        return Err(WalletError::RecoveryPhraseDoesNotMatchMetadata);
    }

    Ok(TestnetWallet {
        wallet_id,
        receive_account,
    })
}
fn create_testnet_wallet_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<TestnetWallet, WalletError> {
    let metadata_path = metadata_path.into();

    if metadata_path.exists() {
        return Err(WalletError::MetadataAlreadyExists);
    }

    let phrase = RecoveryPhrase::generate_24()?;
    vault.store(&wallet_id, &phrase)?;

    let seed = phrase.derive_seed();
    let account = derive_bitcoin_bip84_account(&seed, Network::Testnet)?;
    let receive_account = BitcoinReceiveAccount::new(account);

    let receive_account =
        match PersistentBitcoinReceiveAccount::create(metadata_path, receive_account) {
            Ok(account) => account,
            Err(error) => {
                let _ = vault.delete(&wallet_id);
                return Err(error);
            }
        };

    Ok(TestnetWallet {
        wallet_id,
        receive_account,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    struct RecordingVault {
        stored: Cell<bool>,
    }

    impl RecoveryPhraseVault for RecordingVault {
        fn store(
            &self,
            _wallet_id: &WalletId,
            _phrase: &RecoveryPhrase,
        ) -> Result<(), WalletError> {
            self.stored.set(true);
            Ok(())
        }

        fn load(&self, _wallet_id: &WalletId) -> Result<RecoveryPhrase, WalletError> {
            Err(WalletError::RecoveryPhraseNotFound)
        }

        fn delete(&self, _wallet_id: &WalletId) -> Result<(), WalletError> {
            Ok(())
        }
    }

    const FIRST_TEST_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    const SECOND_TEST_PHRASE: &str =
        "legal winner thank year wave sausage worth useful legal winner thank yellow";

    struct FixedRecoveryVault {
        phrase: &'static str,
    }

    impl RecoveryPhraseVault for FixedRecoveryVault {
        fn store(
            &self,
            _wallet_id: &WalletId,
            _phrase: &RecoveryPhrase,
        ) -> Result<(), WalletError> {
            Err(WalletError::KeychainAccessFailed)
        }

        fn load(&self, _wallet_id: &WalletId) -> Result<RecoveryPhrase, WalletError> {
            RecoveryPhrase::import(self.phrase)
        }

        fn delete(&self, _wallet_id: &WalletId) -> Result<(), WalletError> {
            Ok(())
        }
    }

    fn receive_account_for_phrase(phrase: &str) -> BitcoinReceiveAccount {
        let phrase = RecoveryPhrase::import(phrase).expect("test phrase should be valid");
        let seed = phrase.derive_seed();
        let account = derive_bitcoin_bip84_account(&seed, Network::Testnet)
            .expect("test account should derive");

        BitcoinReceiveAccount::new(account)
    }

    fn ethereum_receive_account_for_phrase(phrase: &str) -> EthereumReceiveAccount {
        let phrase = RecoveryPhrase::import(phrase).expect("test phrase should be valid");
        let seed = phrase.derive_seed();
        let account = derive_ethereum_bip44_account(&seed).expect("Ethereum account should derive");

        EthereumReceiveAccount::new(account)
    }

    #[test]
    fn ethereum_recovery_succeeds_when_phrase_matches_public_metadata() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("ethereum-metadata.json");
        let wallet_id = WalletId::new("ethereum-recovery-test").expect("wallet id should be valid");

        PersistentEthereumReceiveAccount::create(
            path.clone(),
            ethereum_receive_account_for_phrase(FIRST_TEST_PHRASE),
        )
        .expect("metadata should save");

        let vault = FixedRecoveryVault {
            phrase: FIRST_TEST_PHRASE,
        };

        let restored = restore_testnet_ethereum_receive_account_with_vault(wallet_id, path, &vault)
            .expect("matching recovery phrase should restore Ethereum metadata");

        assert_eq!(restored.next_receive_index(), 0);
    }

    #[test]
    fn ethereum_recovery_rejects_phrase_that_does_not_match_public_metadata() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("ethereum-metadata.json");
        let wallet_id =
            WalletId::new("ethereum-recovery-mismatch").expect("wallet id should be valid");

        PersistentEthereumReceiveAccount::create(
            path.clone(),
            ethereum_receive_account_for_phrase(FIRST_TEST_PHRASE),
        )
        .expect("metadata should save");

        let vault = FixedRecoveryVault {
            phrase: SECOND_TEST_PHRASE,
        };

        let result = restore_testnet_ethereum_receive_account_with_vault(wallet_id, path, &vault);

        assert!(matches!(
            result,
            Err(WalletError::RecoveryPhraseDoesNotMatchMetadata)
        ));
    }

    #[test]
    fn setup_stores_the_phrase_and_persists_only_public_metadata() {
        let vault = RecordingVault {
            stored: Cell::new(false),
        };
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("wallet-metadata.json");
        let wallet_id = WalletId::new("test-wallet-01").expect("wallet id should be valid");

        let mut wallet = create_testnet_wallet_with_vault(wallet_id, path.clone(), &vault)
            .expect("testnet wallet should create");
        let address = wallet
            .issue_next_receive_address()
            .expect("receive address should issue");

        assert!(vault.stored.get());
        assert!(path.is_file());
        assert!(address.encoded().starts_with("tb1q"));
        assert_eq!(wallet.next_receive_index(), 1);
    }

    #[test]
    fn recovery_succeeds_when_phrase_matches_public_metadata() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("wallet-metadata.json");
        let wallet_id = WalletId::new("recovery-test-wallet").expect("wallet id should be valid");

        PersistentBitcoinReceiveAccount::create(
            path.clone(),
            receive_account_for_phrase(FIRST_TEST_PHRASE),
        )
        .expect("metadata should save");

        let vault = FixedRecoveryVault {
            phrase: FIRST_TEST_PHRASE,
        };

        let restored = restore_testnet_wallet_with_vault(wallet_id, path, &vault)
            .expect("matching recovery phrase should restore the wallet");

        assert_eq!(restored.next_receive_index(), 0);
    }

    #[test]
    fn recovery_rejects_phrase_that_does_not_match_public_metadata() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("wallet-metadata.json");
        let wallet_id = WalletId::new("mismatch-test-wallet").expect("wallet id should be valid");

        PersistentBitcoinReceiveAccount::create(
            path.clone(),
            receive_account_for_phrase(FIRST_TEST_PHRASE),
        )
        .expect("metadata should save");

        let vault = FixedRecoveryVault {
            phrase: SECOND_TEST_PHRASE,
        };

        let result = restore_testnet_wallet_with_vault(wallet_id, path, &vault);

        assert!(matches!(
            result,
            Err(WalletError::RecoveryPhraseDoesNotMatchMetadata)
        ));
    }

    #[test]
    fn ethereum_setup_derives_and_persists_a_public_receive_account() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("ethereum-metadata.json");
        let wallet_id = WalletId::new("ethereum-test-wallet").expect("wallet id should be valid");
        let vault = FixedRecoveryVault {
            phrase: FIRST_TEST_PHRASE,
        };

        let mut account =
            create_testnet_ethereum_receive_account_with_vault(wallet_id, path.clone(), &vault)
                .expect("Ethereum account should create");

        let address = account
            .issue_next_receive_address()
            .expect("Ethereum address should issue");

        let stored = std::fs::read_to_string(&path).expect("metadata should be readable");

        assert!(path.is_file());
        assert!(address.encoded().starts_with("0x"));
        assert_eq!(address.index(), 0);
        assert_eq!(account.next_receive_index(), 1);
        assert!(!stored.contains(FIRST_TEST_PHRASE));
    }

    #[test]
    fn solana_setup_persists_hardened_receive_addresses() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("solana-metadata.json");
        let wallet_id = WalletId::new("solana-test-wallet").expect("wallet id should be valid");
        let vault = FixedRecoveryVault {
            phrase: FIRST_TEST_PHRASE,
        };

        let first = create_testnet_solana_receive_account_with_vault(
            wallet_id.clone(),
            path.clone(),
            &vault,
        )
        .expect("first Solana address should create");

        let second =
            issue_next_testnet_solana_receive_address_with_vault(wallet_id, path.clone(), &vault)
                .expect("second Solana address should issue");

        let restored = PersistentSolanaReceiveAccount::load(path).expect("metadata should restore");

        assert_eq!(first.account_index(), 0);
        assert_eq!(second.account_index(), 1);
        assert_ne!(first.encoded(), second.encoded());
        assert_eq!(restored.next_account_index(), 2);
    }

    #[test]
    fn dogecoin_setup_derives_and_persists_a_public_receive_account() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("dogecoin-metadata.json");
        let wallet_id = WalletId::new("dogecoin-test-wallet").expect("wallet id should be valid");
        let vault = FixedRecoveryVault {
            phrase: FIRST_TEST_PHRASE,
        };

        let mut account =
            create_testnet_dogecoin_receive_account_with_vault(wallet_id, path.clone(), &vault)
                .expect("Dogecoin account should create");

        let address = account
            .issue_next_receive_address()
            .expect("Dogecoin address should issue");

        let stored = std::fs::read_to_string(&path).expect("metadata should be readable");

        assert!(path.is_file());
        assert_eq!(address.index(), 0);
        assert_eq!(account.next_receive_index(), 1);
        assert!(!stored.contains(FIRST_TEST_PHRASE));
    }

    fn dogecoin_receive_account_for_phrase(phrase: &str) -> DogecoinReceiveAccount {
        let phrase = RecoveryPhrase::import(phrase).expect("test phrase should be valid");
        let seed = phrase.derive_seed();
        let account = derive_dogecoin_bip44_account(&seed, Network::Testnet)
            .expect("Dogecoin account should derive");

        DogecoinReceiveAccount::new(account)
    }

    #[test]
    fn dogecoin_recovery_succeeds_when_phrase_matches_public_metadata() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("dogecoin-metadata.json");
        let wallet_id = WalletId::new("dogecoin-recovery-test").expect("wallet id should be valid");

        PersistentDogecoinReceiveAccount::create(
            path.clone(),
            dogecoin_receive_account_for_phrase(FIRST_TEST_PHRASE),
        )
        .expect("metadata should save");

        let vault = FixedRecoveryVault {
            phrase: FIRST_TEST_PHRASE,
        };

        let restored = restore_testnet_dogecoin_receive_account_with_vault(wallet_id, path, &vault)
            .expect("matching recovery phrase should restore Dogecoin metadata");

        assert_eq!(restored.next_receive_index(), 0);
    }

    #[test]
    fn dogecoin_recovery_rejects_phrase_that_does_not_match_public_metadata() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("dogecoin-metadata.json");
        let wallet_id =
            WalletId::new("dogecoin-recovery-mismatch").expect("wallet id should be valid");

        PersistentDogecoinReceiveAccount::create(
            path.clone(),
            dogecoin_receive_account_for_phrase(FIRST_TEST_PHRASE),
        )
        .expect("metadata should save");

        let vault = FixedRecoveryVault {
            phrase: SECOND_TEST_PHRASE,
        };

        let result = restore_testnet_dogecoin_receive_account_with_vault(wallet_id, path, &vault);

        assert!(matches!(
            result,
            Err(WalletError::RecoveryPhraseDoesNotMatchMetadata)
        ));
    }

    #[test]
    fn solana_recovery_succeeds_when_phrase_matches_public_metadata() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("solana-metadata.json");
        let wallet_id = WalletId::new("solana-recovery-test").expect("wallet id should be valid");

        PersistentSolanaReceiveAccount::create(
            path.clone(),
            solana_receive_account_for_phrase(FIRST_TEST_PHRASE),
        )
        .expect("metadata should save");

        let vault = FixedRecoveryVault {
            phrase: FIRST_TEST_PHRASE,
        };

        let restored = restore_testnet_solana_receive_account_with_vault(wallet_id, path, &vault)
            .expect("matching recovery phrase should restore Solana metadata");

        assert_eq!(restored.next_account_index(), 1);
    }

    #[test]
    fn solana_recovery_rejects_phrase_that_does_not_match_public_metadata() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("solana-metadata.json");
        let wallet_id =
            WalletId::new("solana-recovery-mismatch").expect("wallet id should be valid");

        PersistentSolanaReceiveAccount::create(
            path.clone(),
            solana_receive_account_for_phrase(FIRST_TEST_PHRASE),
        )
        .expect("metadata should save");

        let vault = FixedRecoveryVault {
            phrase: SECOND_TEST_PHRASE,
        };

        let result = restore_testnet_solana_receive_account_with_vault(wallet_id, path, &vault);

        assert!(matches!(
            result,
            Err(WalletError::RecoveryPhraseDoesNotMatchMetadata)
        ));
    }
}

/// Verifies that the Keychain recovery phrase matches all saved Solana addresses.
pub fn restore_testnet_solana_receive_account(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
) -> Result<PersistentSolanaReceiveAccount, WalletError> {
    restore_testnet_solana_receive_account_with_vault(
        wallet_id,
        metadata_path,
        &OsKeychainRecoveryVault,
    )
}

fn restore_testnet_solana_receive_account_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<PersistentSolanaReceiveAccount, WalletError> {
    let receive_account = PersistentSolanaReceiveAccount::load(metadata_path)?;
    let metadata = receive_account.metadata();

    if metadata.network != Network::Testnet {
        return Err(WalletError::WalletNetworkMismatch);
    }

    let phrase = vault.load(&wallet_id)?;
    let seed = phrase.derive_seed();

    for (index, saved_address) in metadata.issued_addresses.iter().enumerate() {
        let index = u32::try_from(index).map_err(|_| WalletError::InvalidStoredMetadata)?;
        let expected_address = derive_solana_bip44_account_address(&seed, index)?;

        if saved_address != &expected_address {
            return Err(WalletError::RecoveryPhraseDoesNotMatchMetadata);
        }
    }

    Ok(receive_account)
}

/// Creates a Solana testnet receive account and allocates its first address.
///
/// Only public address metadata is written to disk.
pub fn create_testnet_solana_receive_account(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
) -> Result<SolanaReceiveAddress, WalletError> {
    create_testnet_solana_receive_account_with_vault(
        wallet_id,
        metadata_path,
        &OsKeychainRecoveryVault,
    )
}

fn create_testnet_solana_receive_account_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<SolanaReceiveAddress, WalletError> {
    let metadata_path = metadata_path.into();

    if metadata_path.exists() {
        return Err(WalletError::MetadataAlreadyExists);
    }

    let phrase = vault.load(&wallet_id)?;
    let seed = phrase.derive_seed();
    let address = derive_solana_bip44_account_address(&seed, 0)?;

    let mut account = SolanaReceiveAccount::new();
    let first_address = account.record_next_receive_address(address)?;

    PersistentSolanaReceiveAccount::create(metadata_path, account)?;

    Ok(first_address)
}

/// Derives and persists the next Solana testnet receive address.
pub fn issue_next_testnet_solana_receive_address(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
) -> Result<SolanaReceiveAddress, WalletError> {
    issue_next_testnet_solana_receive_address_with_vault(
        wallet_id,
        metadata_path,
        &OsKeychainRecoveryVault,
    )
}

fn solana_receive_account_for_phrase(phrase: &str) -> SolanaReceiveAccount {
    let phrase = RecoveryPhrase::import(phrase).expect("test phrase should be valid");
    let seed = phrase.derive_seed();
    let address =
        derive_solana_bip44_account_address(&seed, 0).expect("Solana address should derive");

    let mut account = SolanaReceiveAccount::new();
    account
        .record_next_receive_address(address)
        .expect("Solana address should record");

    account
}

fn issue_next_testnet_solana_receive_address_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<SolanaReceiveAddress, WalletError> {
    let mut account = PersistentSolanaReceiveAccount::load(metadata_path)?;
    let account_index = account.next_account_index();

    let phrase = vault.load(&wallet_id)?;
    let seed = phrase.derive_seed();
    let address = derive_solana_bip44_account_address(&seed, account_index)?;

    account.record_next_receive_address(address)
}
