use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use atomicwrites::{AllowOverwrite, AtomicFile};

use crate::{
    SolanaReceiveAccount, SolanaReceiveAccountMetadata, SolanaReceiveAddress, WalletError,
};

/// Stores public Solana receive-account metadata at one local file path.
pub struct SolanaReceiveAccountStore {
    path: PathBuf,
}

impl SolanaReceiveAccountStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn save(&self, account: &SolanaReceiveAccount) -> Result<(), WalletError> {
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

    fn load(&self) -> Result<SolanaReceiveAccount, WalletError> {
        let serialized = fs::read(&self.path).map_err(|_| WalletError::MetadataStorageFailed)?;

        let metadata: SolanaReceiveAccountMetadata =
            serde_json::from_slice(&serialized).map_err(|_| WalletError::InvalidStoredMetadata)?;

        SolanaReceiveAccount::restore(metadata)
    }
}

/// A Solana receive-account session backed by atomic local metadata storage.
pub struct PersistentSolanaReceiveAccount {
    store: SolanaReceiveAccountStore,
    account: SolanaReceiveAccount,
}

impl PersistentSolanaReceiveAccount {
    pub fn create(
        path: impl Into<PathBuf>,
        account: SolanaReceiveAccount,
    ) -> Result<Self, WalletError> {
        let store = SolanaReceiveAccountStore::new(path);
        store.save(&account)?;

        Ok(Self { store, account })
    }

    pub fn load(path: impl Into<PathBuf>) -> Result<Self, WalletError> {
        let store = SolanaReceiveAccountStore::new(path);
        let account = store.load()?;

        Ok(Self { store, account })
    }

    pub fn metadata(&self) -> SolanaReceiveAccountMetadata {
        self.account.metadata()
    }

    pub const fn next_account_index(&self) -> u32 {
        self.account.next_account_index()
    }

    pub fn record_next_receive_address(
        &mut self,
        address: String,
    ) -> Result<SolanaReceiveAddress, WalletError> {
        let address = self.account.record_next_receive_address(address)?;
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
    use crate::{RecoveryPhrase, derive_solana_bip44_account_address};

    #[test]
    fn stored_account_resumes_at_the_next_unused_hardened_index() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("solana-receive-account.json");

        let phrase = RecoveryPhrase::generate_24().expect("recovery phrase should generate");
        let seed = phrase.derive_seed();

        let first =
            derive_solana_bip44_account_address(&seed, 0).expect("first address should derive");
        let second =
            derive_solana_bip44_account_address(&seed, 1).expect("second address should derive");
        let third =
            derive_solana_bip44_account_address(&seed, 2).expect("third address should derive");

        let mut stored =
            PersistentSolanaReceiveAccount::create(path.clone(), SolanaReceiveAccount::new())
                .expect("initial metadata should save");

        stored
            .record_next_receive_address(first)
            .expect("first address should record");
        stored
            .record_next_receive_address(second)
            .expect("second address should record");

        let mut restored =
            PersistentSolanaReceiveAccount::load(path).expect("stored metadata should load");

        let next = restored
            .record_next_receive_address(third)
            .expect("third address should record");

        assert_eq!(next.account_index(), 2);
        assert_eq!(restored.next_account_index(), 3);
    }

    #[test]
    fn malformed_stored_metadata_is_rejected() {
        let directory = tempfile::tempdir().expect("temporary directory should create");
        let path = directory.path().join("solana-receive-account.json");

        fs::write(&path, b"not valid JSON").expect("test metadata should write");

        let result = PersistentSolanaReceiveAccount::load(path);

        assert!(matches!(result, Err(WalletError::InvalidStoredMetadata)));
    }
}
