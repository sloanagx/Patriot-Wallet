use bdk_wallet::ChangeSet;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{ErrorKind, Write},
    path::PathBuf,
};

use atomicwrites::{AllowOverwrite, AtomicFile};

use crate::WalletError;

/// Public watch-only wallet state. Never contains private keys or seed phrases.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct BitcoinWalletState {
    version: u32,
    changeset: ChangeSet,
}

impl BitcoinWalletState {
    /// Takes the complete accumulated state, not just the latest update.
    pub(crate) fn new(changeset: ChangeSet) -> Self {
        Self {
            version: 1,
            changeset,
        }
    }

    pub(crate) fn into_changeset(self) -> Result<ChangeSet, WalletError> {
        if self.version != 1 {
            return Err(WalletError::InvalidStoredWalletState);
        }

        Ok(self.changeset)
    }
}

pub(crate) struct BitcoinWalletStateStore {
    path: PathBuf,
}

impl BitcoinWalletStateStore {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(crate) fn save(&self, state: &BitcoinWalletState) -> Result<(), WalletError> {
        let serialized =
            serde_json::to_vec(state).map_err(|_| WalletError::WalletStateSerializationFailed)?;

        if let Some(parent) = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|_| WalletError::WalletStateStorageFailed)?;
        }

        AtomicFile::new(&self.path, AllowOverwrite)
            .write(|file| file.write_all(&serialized))
            .map_err(|_| WalletError::WalletStateStorageFailed)?;

        Ok(())
    }

    pub(crate) fn load(&self) -> Result<Option<ChangeSet>, WalletError> {
        let serialized = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(_) => return Err(WalletError::WalletStateStorageFailed),
        };

        let state: BitcoinWalletState = serde_json::from_slice(&serialized)
            .map_err(|_| WalletError::InvalidStoredWalletState)?;

        state.into_changeset().map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_state_version_is_rejected() {
        let state = BitcoinWalletState {
            version: 2,
            changeset: ChangeSet::default(),
        };

        assert!(matches!(
            state.into_changeset(),
            Err(WalletError::InvalidStoredWalletState)
        ));
    }

    #[test]
    fn missing_state_returns_none() {
        let directory = tempfile::tempdir().unwrap();
        let store = BitcoinWalletStateStore::new(directory.path().join("state.json"));

        assert!(store.load().unwrap().is_none());
    }

    #[test]
    fn saved_state_preserves_network_and_chain_checkpoint() {
        let directory = tempfile::tempdir().unwrap();
        let store = BitcoinWalletStateStore::new(directory.path().join("nested/state.json"));

        let mut changeset = ChangeSet::default();
        changeset.network = Some(bitcoin::Network::Testnet);
        changeset.local_chain.blocks.insert(
            0,
            Some(
                bitcoin::blockdata::constants::genesis_block(bitcoin::Network::Testnet)
                    .block_hash(),
            ),
        );

        store
            .save(&BitcoinWalletState::new(changeset.clone()))
            .unwrap();

        assert_eq!(store.load().unwrap(), Some(changeset));
    }

    #[test]
    fn malformed_state_is_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.json");
        fs::write(&path, b"not valid JSON").unwrap();

        let store = BitcoinWalletStateStore::new(path);

        assert!(matches!(
            store.load(),
            Err(WalletError::InvalidStoredWalletState)
        ));
    }
}
