use crate::{AddressValidator, Asset, AssetId, Blockchain, Network, WalletError};
use bitcoin::{
    Network as BitcoinNetwork,
    bip32::{ChildNumber, Xpriv, Xpub},
    hashes::{Hash, hash160, sha256d},
    secp256k1::Secp256k1,
};
use std::str::FromStr;

use bitcoin::NetworkKind;
use serde::{Deserialize, Serialize};

/// Validates Dogecoin P2PKH and P2SH Base58Check recipient addresses.
pub struct DogecoinAddressValidator;

/// Public Dogecoin BIP-44 account data for watch-only address derivation.

/// An allocated Dogecoin receive address and its BIP-44 address index.
pub struct DogecoinReceiveAddress {
    index: u32,
    value: String,
}

impl DogecoinReceiveAddress {
    pub const fn index(&self) -> u32 {
        self.index
    }

    pub fn encoded(&self) -> &str {
        &self.value
    }
}
pub struct DogecoinAccountXpub {
    xpub: Xpub,
    network: Network,
}

impl DogecoinAccountXpub {
    pub fn encoded(&self) -> String {
        self.xpub.to_string()
    }

    pub(crate) const fn network(&self) -> Network {
        self.network
    }

    pub(crate) fn import(encoded: &str, network: Network) -> Result<Self, WalletError> {
        let xpub = Xpub::from_str(encoded).map_err(|_| WalletError::InvalidWatchOnlyAccount)?;

        let expected_network = match network {
            Network::Mainnet => NetworkKind::Main,
            Network::Testnet => NetworkKind::Test,
        };

        if xpub.network != expected_network {
            return Err(WalletError::WatchOnlyAccountNetworkMismatch);
        }

        Ok(Self { xpub, network })
    }

    pub fn receive_address_at(&self, index: u32) -> Result<DogecoinReceiveAddress, WalletError> {
        let value = self.derive_receive_address(index)?;

        Ok(DogecoinReceiveAddress { index, value })
    }

    pub fn derive_receive_address(&self, index: u32) -> Result<String, WalletError> {
        let change =
            ChildNumber::from_normal_idx(0).map_err(|_| WalletError::KeyDerivationFailed)?;
        let child =
            ChildNumber::from_normal_idx(index).map_err(|_| WalletError::KeyDerivationFailed)?;

        let secp = Secp256k1::new();
        let derived = self
            .xpub
            .derive_pub(&secp, &[change, child])
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        Ok(dogecoin_p2pkh_address(
            &derived.public_key.serialize(),
            self.network,
        ))
    }
}

const DOGECOIN_METADATA_FORMAT_VERSION: u8 = 1;

/// Versioned public metadata for Dogecoin receive-address allocation.
#[derive(Serialize, Deserialize)]
pub struct DogecoinReceiveAccountMetadata {
    pub format_version: u8,
    pub account_xpub: String,
    pub network: Network,
    pub next_receive_address_index: u32,
}

/// Allocates unique Dogecoin BIP-44 receive addresses.
pub struct DogecoinReceiveAccount {
    account: DogecoinAccountXpub,
    next_receive_index: u32,
}

impl DogecoinReceiveAccount {
    pub fn new(account: DogecoinAccountXpub) -> Self {
        Self {
            account,
            next_receive_index: 0,
        }
    }

    pub const fn next_receive_index(&self) -> u32 {
        self.next_receive_index
    }

    pub fn metadata(&self) -> DogecoinReceiveAccountMetadata {
        DogecoinReceiveAccountMetadata {
            format_version: DOGECOIN_METADATA_FORMAT_VERSION,
            account_xpub: self.account.encoded(),
            network: self.account.network(),
            next_receive_address_index: self.next_receive_index,
        }
    }

    pub fn restore(metadata: DogecoinReceiveAccountMetadata) -> Result<Self, WalletError> {
        if metadata.format_version != DOGECOIN_METADATA_FORMAT_VERSION {
            return Err(WalletError::UnsupportedMetadataVersion);
        }

        let account = DogecoinAccountXpub::import(&metadata.account_xpub, metadata.network)?;

        Ok(Self {
            account,
            next_receive_index: metadata.next_receive_address_index,
        })
    }

    pub fn issue_next_receive_address(&mut self) -> Result<DogecoinReceiveAddress, WalletError> {
        let index = self.next_receive_index;

        if index >= (1 << 31) {
            return Err(WalletError::ReceiveAddressIndexExhausted);
        }

        let address = self.account.receive_address_at(index)?;

        self.next_receive_index = self
            .next_receive_index
            .checked_add(1)
            .ok_or(WalletError::ReceiveAddressIndexExhausted)?;

        Ok(address)
    }
}

impl AddressValidator for DogecoinAddressValidator {
    fn validate(&self, asset: AssetId, network: Network, address: &str) -> Result<(), WalletError> {
        if asset.asset != Asset::Dogecoin || asset.chain != Blockchain::Dogecoin {
            return Err(WalletError::InvalidAddress);
        }

        let decoded = bs58::decode(address)
            .into_vec()
            .map_err(|_| WalletError::InvalidAddress)?;

        if decoded.len() != 25 {
            return Err(WalletError::InvalidAddress);
        }

        let (payload, checksum) = decoded.split_at(21);
        let expected_checksum = sha256d::Hash::hash(payload).to_byte_array();

        if checksum != &expected_checksum[..4] {
            return Err(WalletError::InvalidAddress);
        }

        let valid_prefix = match network {
            Network::Mainnet => matches!(payload[0], 0x1e | 0x16),
            Network::Testnet => matches!(payload[0], 0x71 | 0xc4),
        };

        if !valid_prefix {
            return Err(WalletError::AddressNetworkMismatch);
        }

        Ok(())
    }
}

/// Derives the Dogecoin BIP-44 account at `m/44'/3'/0'`.
pub fn derive_dogecoin_bip44_account(
    seed: &crate::SeedMaterial,
    network: Network,
) -> Result<DogecoinAccountXpub, WalletError> {
    let purpose =
        ChildNumber::from_hardened_idx(44).map_err(|_| WalletError::KeyDerivationFailed)?;
    let coin_type =
        ChildNumber::from_hardened_idx(3).map_err(|_| WalletError::KeyDerivationFailed)?;
    let account =
        ChildNumber::from_hardened_idx(0).map_err(|_| WalletError::KeyDerivationFailed)?;

    let bitcoin_network = match network {
        Network::Mainnet => BitcoinNetwork::Bitcoin,
        Network::Testnet => BitcoinNetwork::Testnet,
    };

    seed.with_bytes(|seed_bytes| {
        let secp = Secp256k1::new();
        let master = Xpriv::new_master(bitcoin_network, seed_bytes)
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        let derived = master
            .derive_priv(&secp, &[purpose, coin_type, account])
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        Ok(DogecoinAccountXpub {
            xpub: Xpub::from_priv(&secp, &derived),
            network,
        })
    })
}

fn dogecoin_p2pkh_address(public_key: &[u8], network: Network) -> String {
    let prefix = match network {
        Network::Mainnet => 0x1e,
        Network::Testnet => 0x71,
    };

    let mut payload = vec![prefix];
    payload.extend_from_slice(&hash160::Hash::hash(public_key).to_byte_array());

    let checksum = sha256d::Hash::hash(&payload).to_byte_array();
    payload.extend_from_slice(&checksum[..4]);

    bs58::encode(payload).into_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RecipientAddress, RecoveryPhrase};

    fn dogecoin_asset() -> AssetId {
        AssetId::new(Asset::Dogecoin, Blockchain::Dogecoin).expect("valid Dogecoin asset")
    }

    fn address_with_prefix(prefix: u8) -> String {
        let mut payload = vec![prefix];
        payload.extend_from_slice(&[7_u8; 20]);

        let checksum = sha256d::Hash::hash(&payload).to_byte_array();
        payload.extend_from_slice(&checksum[..4]);

        bs58::encode(payload).into_string()
    }

    #[test]
    fn accepts_a_valid_mainnet_dogecoin_address() {
        let address = address_with_prefix(0x1e);

        let result = RecipientAddress::parse(
            dogecoin_asset(),
            Network::Mainnet,
            address,
            &DogecoinAddressValidator,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_a_testnet_address_when_mainnet_is_selected() {
        let testnet_address = address_with_prefix(0x71);

        let result =
            DogecoinAddressValidator.validate(dogecoin_asset(), Network::Mainnet, &testnet_address);

        assert_eq!(result, Err(WalletError::AddressNetworkMismatch));
    }

    #[test]
    fn rejects_an_address_with_an_invalid_checksum() {
        let mut address = address_with_prefix(0x1e);
        address.pop();
        address.push('1');

        let result =
            DogecoinAddressValidator.validate(dogecoin_asset(), Network::Mainnet, &address);

        assert_eq!(result, Err(WalletError::InvalidAddress));
    }

    #[test]
    fn derives_distinct_valid_testnet_dogecoin_addresses() {
        let phrase = RecoveryPhrase::generate_24().expect("recovery phrase should generate");
        let seed = phrase.derive_seed();

        let account = derive_dogecoin_bip44_account(&seed, Network::Testnet)
            .expect("Dogecoin account should derive");

        let first = account
            .derive_receive_address(0)
            .expect("first Dogecoin address should derive");
        let second = account
            .derive_receive_address(1)
            .expect("second Dogecoin address should derive");

        assert_ne!(first, second);

        let result = DogecoinAddressValidator.validate(dogecoin_asset(), Network::Testnet, &first);

        assert!(result.is_ok());
    }
}
