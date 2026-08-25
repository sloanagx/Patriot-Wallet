use std::{fmt, str::FromStr};

use bitcoin::{
    Network as BitcoinNetwork,
    bip32::{DerivationPath, Xpriv, Xpub},
    secp256k1::Secp256k1,
};

use crate::{Network, SeedMaterial, WalletError};

/// Watch only Bitcoin BIP-84 account data
///
/// This contains no private key, but is still privacy sensitive because it
/// lets software discover addresses associated with this account.
pub struct BitcoinAccountXpub(Xpub);

impl BitcoinAccountXpub {
    /// Returns the serialized extended public key for watch-only use.
    pub fn encoded(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Debug for BitcoinAccountXpub {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BitcoinAccountXpub([REDACTED])")
    }
}

pub fn derive_bitcoin_bip84_account(
    seed: &SeedMaterial,
    network: Network,
) -> Result<BitcoinAccountXpub, WalletError> {
    let (bitcoin_network, derivation_path) = match network {
        Network::Mainnet => (BitcoinNetwork::Bitcoin, "m/84'/0'/0'"),
        Network::Testnet => (BitcoinNetwork::Testnet, "m/84'/1'/0'"),
    };

    let path =
        DerivationPath::from_str(derivation_path).map_err(|_| WalletError::KeyDerivationFailed)?;

    seed.with_bytes(|seed_bytes| {
        let secp = Secp256k1::new();

        let master = Xpriv::new_master(bitcoin_network, seed_bytes)
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        let account = master
            .derive_priv(&secp, &path)
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        Ok(BitcoinAccountXpub(Xpub::from_priv(&secp, &account)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecoveryPhrase;

    #[test]
    fn testnet_bip84_account_is_encoded_as_a_test_public_key() {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");
        let seed = phrase.derive_seed();

        let account = derive_bitcoin_bip84_account(&seed, Network::Testnet)
            .expect("testnet account derivation should succeed");

        assert!(account.encoded().starts_with("tpub"));
    }

    #[test]
    fn account_debug_output_is_redacted() {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");
        let seed = phrase.derive_seed();

        let account = derive_bitcoin_bip84_account(&seed, Network::Testnet)
            .expect("testnet account derivation should succeed");

        assert_eq!(format!("{account:?}"), "BitcoinAccountXpub([REDACTED])");
    }
}
