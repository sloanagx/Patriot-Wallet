use std::fmt;

use bip39::Mnemonic;

use crate::WalletError;

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
}

impl fmt::Debug for RecoveryPhrase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RecoveryPhrase([REDACTED])")
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
}
