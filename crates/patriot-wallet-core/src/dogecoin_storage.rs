use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use atomicwrites::{AllowOverwrite, AtomicFile};

use crate::{
    DogecoinReceiveAccount, DogecoinReceiveAccountMetadata, DogecoinReceiveAddress, WalletError,
};

/// Stores public Dogecoin receive-account metadata at one local file path.
pub struct DogecoinReceiveAccountStore {
    path: PathBuf,
}

impl DogecoinReceiveAccountStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn save(&self, account: &DogecoinReceiveAccount) -> Result<(), WalletError> {
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

    fn load(&self) -> Result<DogecoinReceiveAccount, WalletError> {
        let serialized = fs::read(&self.path).map_err(|_| WalletError::MetadataStorageFailed)?;

        let metadata: DogecoinReceiveAccountMetadata =
            serde_json::from_slice(&serialized).map_err(|_| WalletError::InvalidStoredMetadata)?;

        DogecoinReceiveAccount::restore(metadata)
    }
}

/// A Dogecoin receive-account session backed by atomic local metadata storage.
pub struct PersistentDogecoinReceiveAccount {
    store: DogecoinReceiveAccountStore,
    account: DogecoinReceiveAccount,
}

impl PersistentDogecoinReceiveAccount {
    pub fn create(
        path: impl Into<PathBuf>,
        account: DogecoinReceiveAccount,
    ) -> Result<Self, WalletError> {
        let store = DogecoinReceiveAccountStore::new(path);
        store.save(&account)?;

        Ok(Self { store, account })
    }

    pub fn load(path: impl Into<PathBuf>) -> Result<Self, WalletError> {
        let store = DogecoinReceiveAccountStore::new(path);
        let account = store.load()?;

        Ok(Self { store, account })
    }

    pub fn metadata(&self) -> DogecoinReceiveAccountMetadata {
        self.account.metadata()
    }

    pub const fn next_receive_index(&self) -> u32 {
        self.account.next_receive_index()
    }

    pub fn issue_next_receive_address(&mut self) -> Result<DogecoinReceiveAddress, WalletError> {
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
    use crate::{Network, RecoveryPhrase, derive_dogecoin_bip44_account};
    fn testnet_account() -> DogecoinReceiveAccount {
        let phrase = RecoveryPhrase::generate_24().expect("recovery phrase should generate");
        let seed = phrase.derive_seed();
        let account = derive_dogecoin_bip44_account(&seed, Network::Testnet)
            .expect("Dogecoin account should derive");

        DogecoinReceiveAccount::new(account)
    }

    #[test]
    fn stored_account_resumes_at_the_next_unused_address_index() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("dogecoin-receive-account.json");

        let mut stored = PersistentDogecoinReceiveAccount::create(path.clone(), testnet_account())
            .expect("initial metadata should save");

        stored
            .issue_next_receive_address()
            .expect("first address should issue");
        stored
            .issue_next_receive_address()
            .expect("second address should issue");

        let mut restored =
            PersistentDogecoinReceiveAccount::load(path).expect("stored metadata should load");

        let next = restored
            .issue_next_receive_address()
            .expect("third address should issue");

        assert_eq!(next.index(), 2);
        assert_eq!(restored.next_receive_index(), 3);
    }

    #[test]
    fn malformed_stored_metadata_is_rejected() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("dogecoin-receive-account.json");

        fs::write(&path, b"not valid JSON").expect("test metadata should write");

        let result = PersistentDogecoinReceiveAccount::load(path);

        assert!(matches!(result, Err(WalletError::InvalidStoredMetadata)));
    }
}
