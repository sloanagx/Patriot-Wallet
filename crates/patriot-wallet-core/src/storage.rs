use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use atomicwrites::{AllowOverwrite, AtomicFile};

use crate::{
    BdkBitcoinWatchOnlyWallet, BitcoinReceiveAccount, BitcoinReceiveAccountMetadata,
    BitcoinReceiveAddress, WalletError,
};

/// Stores public Bitcoin receive-account metadata at one local file path.
///
/// The file contains no seed phrase or private key.
pub struct BitcoinReceiveAccountStore {
    path: PathBuf,
}

impl BitcoinReceiveAccountStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn save(&self, account: &BitcoinReceiveAccount) -> Result<(), WalletError> {
        let serialized = serde_json::to_vec(&account.metadata())
            .map_err(|_| WalletError::MetadataSerializationFailed)?;

        if let Some(parent) = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|_| WalletError::MetadataStorageFailed)?;
        }

        let file = AtomicFile::new(&self.path, AllowOverwrite);
        file.write(|temporary_file| temporary_file.write_all(&serialized))
            .map_err(|_| WalletError::MetadataStorageFailed)?;

        Ok(())
    }

    fn load(&self) -> Result<BitcoinReceiveAccount, WalletError> {
        let serialized = fs::read(&self.path).map_err(|_| WalletError::MetadataStorageFailed)?;
        let metadata: BitcoinReceiveAccountMetadata =
            serde_json::from_slice(&serialized).map_err(|_| WalletError::InvalidStoredMetadata)?;

        BitcoinReceiveAccount::restore(metadata)
    }
}

/// A receive-account session backed by atomic local metadata storage.
pub struct PersistentBitcoinReceiveAccount {
    store: BitcoinReceiveAccountStore,
    account: BitcoinReceiveAccount,
}

impl PersistentBitcoinReceiveAccount {
    /// Creates the initial metadata file before allowing an address to be issued.
    pub fn create(
        path: impl Into<PathBuf>,
        account: BitcoinReceiveAccount,
    ) -> Result<Self, WalletError> {
        let store = BitcoinReceiveAccountStore::new(path);
        store.save(&account)?;

        Ok(Self { store, account })
    }

    pub fn load(path: impl Into<PathBuf>) -> Result<Self, WalletError> {
        let store = BitcoinReceiveAccountStore::new(path);
        let account = store.load()?;

        Ok(Self { store, account })
    }

    pub fn metadata(&self) -> BitcoinReceiveAccountMetadata {
        self.account.metadata()
    }

    pub fn watch_only_wallet(&self) -> Result<BdkBitcoinWatchOnlyWallet, WalletError> {
        self.account.watch_only_wallet()
    }

    pub const fn next_receive_index(&self) -> u32 {
        self.account.next_receive_index()
    }

    /// Reserves an address, persists the advanced index atomically, then returns it.
    pub fn issue_next_receive_address(&mut self) -> Result<BitcoinReceiveAddress, WalletError> {
        let address = self.account.issue_next_receive_address()?;
        self.store.save(&self.account)?;

        Ok(address)
    }

    pub fn metadata_path(&self) -> &Path {
        &self.store.path
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
            .expect("testnet account derivation should succeed");

        BitcoinReceiveAccount::new(account)
    }

    #[test]
    fn stored_account_resumes_at_the_next_unused_address_index() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("bitcoin-receive-account.json");

        let mut stored = PersistentBitcoinReceiveAccount::create(path.clone(), testnet_account())
            .expect("initial metadata should save");

        stored
            .issue_next_receive_address()
            .expect("first address should issue");
        stored
            .issue_next_receive_address()
            .expect("second address should issue");

        let mut restored =
            PersistentBitcoinReceiveAccount::load(path).expect("stored metadata should load");

        let next = restored
            .issue_next_receive_address()
            .expect("third address should issue");

        assert_eq!(next.index(), 2);
        assert_eq!(restored.next_receive_index(), 3);
    }

    #[test]
    fn malformed_stored_metadata_is_rejected() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("bitcoin-receive-account.json");

        fs::write(&path, b"not valid JSON").expect("test metadata should write");

        let result = PersistentBitcoinReceiveAccount::load(path);

        assert!(matches!(result, Err(WalletError::InvalidStoredMetadata)));
    }
}
