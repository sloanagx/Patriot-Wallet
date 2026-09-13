use keyring::Entry;
use zeroize::Zeroizing;

use crate::{RecoveryPhrase, WalletError};

const KEYCHAIN_SERVICE: &str = "Patriot Wallet";

/// A non-secret identifier used to locate one wallet's recovery phrase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalletId(String);

impl WalletId {
    pub fn new(value: impl Into<String>) -> Result<Self, WalletError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));

        if valid {
            Ok(Self(value))
        } else {
            Err(WalletError::InvalidWalletId)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The minimal secret-storage contract used by wallet setup and recovery flows.
pub trait RecoveryPhraseVault {
    fn store(&self, wallet_id: &WalletId, phrase: &RecoveryPhrase) -> Result<(), WalletError>;
    fn load(&self, wallet_id: &WalletId) -> Result<RecoveryPhrase, WalletError>;
    fn delete(&self, wallet_id: &WalletId) -> Result<(), WalletError>;
}

/// Stores recovery phrases in the operating system's credential vault.
///
/// On macOS this uses Keychain; on Windows it uses Credential Manager.
pub struct OsKeychainRecoveryVault;

impl OsKeychainRecoveryVault {
    fn entry(wallet_id: &WalletId) -> Result<Entry, WalletError> {
        let username = format!("wallet:{}:bip39", wallet_id.as_str());
        Entry::new(KEYCHAIN_SERVICE, &username).map_err(|_| WalletError::KeychainAccessFailed)
    }
}

impl RecoveryPhraseVault for OsKeychainRecoveryVault {
    fn store(&self, wallet_id: &WalletId, phrase: &RecoveryPhrase) -> Result<(), WalletError> {
        let entry = Self::entry(wallet_id)?;

        phrase.with_words(|words| {
            entry
                .set_password(words)
                .map_err(|_| WalletError::KeychainAccessFailed)
        })
    }

    fn load(&self, wallet_id: &WalletId) -> Result<RecoveryPhrase, WalletError> {
        let entry = Self::entry(wallet_id)?;
        let words = match entry.get_password() {
            Ok(words) => Zeroizing::new(words),
            Err(keyring::Error::NoEntry) => return Err(WalletError::RecoveryPhraseNotFound),
            Err(_) => return Err(WalletError::KeychainAccessFailed),
        };

        RecoveryPhrase::import(&words).map_err(|_| WalletError::InvalidStoredRecoveryPhrase)
    }

    fn delete(&self, wallet_id: &WalletId) -> Result<(), WalletError> {
        let entry = Self::entry(wallet_id)?;

        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Err(WalletError::RecoveryPhraseNotFound),
            Err(_) => Err(WalletError::KeychainAccessFailed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_id_accepts_safe_identifiers() {
        let id = WalletId::new("test-wallet_01").expect("identifier should be valid");

        assert_eq!(id.as_str(), "test-wallet_01");
    }

    #[test]
    fn wallet_id_rejects_empty_or_unsafe_identifiers() {
        assert!(matches!(
            WalletId::new(""),
            Err(WalletError::InvalidWalletId)
        ));
        assert!(matches!(
            WalletId::new("wallet/one"),
            Err(WalletError::InvalidWalletId)
        ));
    }
}
