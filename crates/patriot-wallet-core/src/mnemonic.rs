use std::fmt;

use bip39::Mnemonic;

use crate::WalletError;

use zeroize::Zeroizing;

/// A BIP-39 recovery phrase.
///
/// This type intentionally does not implement `Display` or expose its words.
/// The underlying BIP-39 value uses memory clearing when dropped.
pub struct RecoveryPhrase(Mnemonic);

impl RecoveryPhrase {
    /// Generates a new 24-word English recovery phrase.
    pub fn generate_24() -> Result<Self, WalletError> {
        Mnemonic::generate(24)
            .map(Self)
            .map_err(|_| WalletError::MnemonicGenerationFailed)
    }

    /// Derives seed material without a BIP-39 passphrase.
    ///
    /// Optional passphrase support will be added only with secure GUI input.
    pub fn derive_seed(&self) -> SeedMaterial {
        SeedMaterial(Zeroizing::new(self.0.to_seed("")))
    }

    /// Imports and checksum-validates an existing recovery phrase.
    pub fn import(phrase: &str) -> Result<Self, WalletError> {
        if phrase.is_empty() || phrase.trim() != phrase {
            return Err(WalletError::InvalidMnemonic);
        }

        Mnemonic::parse(phrase)
            .map(Self)
            .map_err(|_| WalletError::InvalidMnemonic)
    }

    pub fn word_count(&self) -> usize {
        self.0.word_count()
    }

    /// Makes the phrase available only to internal secret-storage or key-derivation code.
    pub(crate) fn with_words<T>(&self, operation: impl FnOnce(&str) -> T) -> T {
        let words = Zeroizing::new(self.0.to_string());
        operation(&words)
    }
}

impl fmt::Debug for RecoveryPhrase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RecoveryPhrase([REDACTED])")
    }
}

/// Derived BIP-39 seed material held in zeroizing memory.
///
/// It intentionally has no method that exposes its bytes publicly.
///
/// The backing field is kept intentionally so the zeroizing buffer is retained
/// and securely cleared on drop; it is not read directly elsewhere.
#[allow(dead_code)]
pub struct SeedMaterial(Zeroizing<[u8; 64]>);

impl SeedMaterial {
    pub const fn byte_len(&self) -> usize {
        64
    }

    pub(crate) fn with_bytes<T>(&self, operation: impl FnOnce(&[u8; 64]) -> T) -> T {
        operation(&self.0)
    }
}

impl fmt::Debug for SeedMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SeedMaterial([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_recovery_phrase_has_24_words() {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");

        assert_eq!(phrase.word_count(), 24);
    }

    #[test]
    fn malformed_recovery_phrase_is_rejected() {
        let result = RecoveryPhrase::import("this is not a valid recovery phrase");

        assert!(matches!(result, Err(WalletError::InvalidMnemonic)));
    }

    #[test]
    fn debug_output_redacts_the_recovery_phrase() {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");

        assert_eq!(format!("{phrase:?}"), "RecoveryPhrase([REDACTED])");
    }

    #[test]
    fn derived_seed_has_the_bip39_length() {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");
        let seed = phrase.derive_seed();

        assert_eq!(seed.byte_len(), 64);
    }

    #[test]
    fn seed_debug_output_is_redacted() {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");
        let seed = phrase.derive_seed();

        assert_eq!(format!("{seed:?}"), "SeedMaterial([REDACTED])");
    }
}
