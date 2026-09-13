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

fn create_testnet_wallet_with_vault(
    wallet_id: WalletId,
    metadata_path: impl Into<PathBuf>,
    vault: &impl RecoveryPhraseVault,
) -> Result<TestnetWallet, WalletError> {
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
}
