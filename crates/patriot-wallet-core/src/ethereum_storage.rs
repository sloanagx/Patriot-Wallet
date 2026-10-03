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

    pub fn issue_next_receive_address(&mut self) -> Result<EthereumReceiveAddress, WalletError> {
        let address = self.account.issue_next_receive_address()?;
        self.store.save(&self.account)?;

        Ok(address)
    }

    pub fn metadata_path(&self) -> &Path {
        &self.store.path
    }
}
