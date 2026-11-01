use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use atomicwrites::{AllowOverwrite, AtomicFile};

use crate::{
    EthereumReceiveAccount, EthereumReceiveAccountMetadata, EthereumReceiveAddress, WalletError,
};

/// Stores public Ethereum receive-account metadata at one local file path.
pub struct EthereumReceiveAccountStore {
    path: PathBuf,
}

impl EthereumReceiveAccountStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn save(&self, account: &EthereumReceiveAccount) -> Result<(), WalletError> {
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

    fn load(&self) -> Result<EthereumReceiveAccount, WalletError> {
        let serialized = fs::read(&self.path).map_err(|_| WalletError::MetadataStorageFailed)?;

        let metadata: EthereumReceiveAccountMetadata =
            serde_json::from_slice(&serialized).map_err(|_| WalletError::InvalidStoredMetadata)?;

        EthereumReceiveAccount::restore(metadata)
    }
}

/// An Ethereum receive-account session backed by atomic local metadata storage.
pub struct PersistentEthereumReceiveAccount {
    store: EthereumReceiveAccountStore,
    account: EthereumReceiveAccount,
}

impl PersistentEthereumReceiveAccount {
    pub fn create(
        path: impl Into<PathBuf>,
        account: EthereumReceiveAccount,
    ) -> Result<Self, WalletError> {
        let store = EthereumReceiveAccountStore::new(path);
        store.save(&account)?;

        Ok(Self { store, account })
    }

    pub fn load(path: impl Into<PathBuf>) -> Result<Self, WalletError> {
        let store = EthereumReceiveAccountStore::new(path);
        let account = store.load()?;

        Ok(Self { store, account })
    }

    pub fn metadata(&self) -> EthereumReceiveAccountMetadata {
        self.account.metadata()
    }

    pub const fn next_receive_index(&self) -> u32 {
        self.account.next_receive_index()
    }

    pub fn last_issued_receive_address(&self) -> Result<EthereumReceiveAddress, WalletError> {
        self.account.last_issued_receive_address()
    }

    pub fn issue_next_receive_address(&mut self) -> Result<EthereumReceiveAddress, WalletError> {
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
    use std::fs;

    use super::*;
    use crate::{RecoveryPhrase, derive_ethereum_bip44_account};

    fn testnet_account() -> EthereumReceiveAccount {
        let phrase = RecoveryPhrase::generate_24().expect("recovery phrase should generate");
        let seed = phrase.derive_seed();
        let account = derive_ethereum_bip44_account(&seed).expect("Ethereum account should derive");

        EthereumReceiveAccount::new(account)
    }

    #[test]
    fn stored_account_resumes_at_the_next_unused_address_index() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("ethereum-receive-account.json");

        let mut stored = PersistentEthereumReceiveAccount::create(path.clone(), testnet_account())
            .expect("initial metadata should save");

        stored
            .issue_next_receive_address()
            .expect("first address should issue");
        stored
            .issue_next_receive_address()
            .expect("second address should issue");

        let mut restored =
            PersistentEthereumReceiveAccount::load(path).expect("stored metadata should load");

        let next = restored
            .issue_next_receive_address()
            .expect("third address should issue");

        assert_eq!(next.index(), 2);
        assert_eq!(restored.next_receive_index(), 3);
    }

    #[test]
    fn malformed_stored_metadata_is_rejected() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("ethereum-receive-account.json");

        fs::write(&path, b"not valid JSON").expect("test metadata should write");

        let result = PersistentEthereumReceiveAccount::load(path);

        assert!(matches!(result, Err(WalletError::InvalidStoredMetadata)));
    }

    #[test]
    fn reading_the_last_issued_address_does_not_advance_the_index() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("ethereum-receive-account.json");

        let mut stored = PersistentEthereumReceiveAccount::create(path, testnet_account())
            .expect("initial metadata should save");

        let issued = stored
            .issue_next_receive_address()
            .expect("first address should issue");

        let displayed = stored
            .last_issued_receive_address()
            .expect("issued address should be readable");

        assert_eq!(displayed.index(), issued.index());
        assert_eq!(displayed.encoded(), issued.encoded());
        assert_eq!(stored.next_receive_index(), 1);
    }
}
