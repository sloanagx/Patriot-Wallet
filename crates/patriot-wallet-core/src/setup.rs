use std::path::PathBuf;

use crate::Network;
use crate::{
    BitcoinReceiveAccount, BitcoinReceiveAddress, OsKeychainRecoveryVault,
    PersistentBitcoinReceiveAccount, RecoveryPhrase, RecoveryPhraseVault, WalletError, WalletId,
    derive_bitcoin_bip84_account,
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
}
