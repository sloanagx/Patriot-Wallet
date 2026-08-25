use std::{fmt, str::FromStr};

use bitcoin::{
    Address as BitcoinAddress, Network as BitcoinNetwork,
    bip32::{ChildNumber, DerivationPath, Xpriv, Xpub},
    secp256k1::Secp256k1,
};

use crate::{Network, SeedMaterial, WalletError};

/// Watch only Bitcoin BIP-84 account data
///
/// This contains no private key, but is still privacy sensitive because it
/// lets software discover addresses associated with this account.
pub struct BitcoinAccountXpub {
    xpub: Xpub,
    network: Network,
}

/// A public Bitcoin receive address derived from a BIP-84 account.
pub struct BitcoinReceiveAddress {
    index: u32,
    value: BitcoinAddress,
}

impl BitcoinReceiveAddress {
    pub const fn index(&self) -> u32 {
        self.index
    }

    pub fn encoded(&self) -> String {
        self.value.to_string()
    }
}

impl fmt::Debug for BitcoinReceiveAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BitcoinReceiveAddress([REDACTED])")
    }
}

impl BitcoinAccountXpub {
    pub fn encoded(&self) -> String {
        self.xpub.to_string()
    }

    pub fn derive_receive_address(&self, index: u32) -> Result<BitcoinReceiveAddress, WalletError> {
        let change =
            ChildNumber::from_normal_idx(0).map_err(|_| WalletError::KeyDerivationFailed)?;
        let child =
            ChildNumber::from_normal_idx(index).map_err(|_| WalletError::KeyDerivationFailed)?;

        let secp = Secp256k1::new();
        let derived = self
            .xpub
            .derive_pub(&secp, &[change, child])
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        let bitcoin_network = match self.network {
            Network::Mainnet => BitcoinNetwork::Bitcoin,
            Network::Testnet => BitcoinNetwork::Testnet,
        };

        Ok(BitcoinReceiveAddress {
            index,
            value: BitcoinAddress::p2wpkh(&derived.to_pub(), bitcoin_network),
        })
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

        Ok(BitcoinAccountXpub {
            xpub: Xpub::from_priv(&secp, &account),
            network,
        })
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
    #[test]
    fn testnet_account_derives_native_segwit_receive_addresses() {
        let phrase = RecoveryPhrase::generate_24().expect("generation should succeed");
        let seed = phrase.derive_seed();

        let account = derive_bitcoin_bip84_account(&seed, Network::Testnet)
            .expect("testnet account derivation should succeed");

        let first = account
            .derive_receive_address(0)
            .expect("first receive address should derive");
        let second = account
            .derive_receive_address(1)
            .expect("second receive address should derive");

        assert_eq!(first.index(), 0);
        assert!(first.encoded().starts_with("tb1q"));
        assert_ne!(first.encoded(), second.encoded());
    }
}
