use crate::{AddressValidator, Asset, AssetId, Blockchain, Network, WalletError};
use bitcoin::NetworkKind;
use bitcoin::{
    Network as BitcoinNetwork,
    bip32::{ChildNumber, Xpriv, Xpub},
    secp256k1::Secp256k1,
};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use std::str::FromStr;

/// Validates Ethereum addresses, including EIP-55 mixed-case checksums.
pub struct EthereumAddressValidator;
/// Public Ethereum BIP-44 account data for watch-only address derivation.
/// An allocated Ethereum receive address and its BIP-44 address index.
pub struct EthereumReceiveAddress {
    index: u32,
    value: String,
}

impl EthereumReceiveAddress {
    pub const fn index(&self) -> u32 {
        self.index
    }

    pub fn encoded(&self) -> &str {
        &self.value
    }
}
pub struct EthereumAccountXpub {
    xpub: Xpub,
}

impl EthereumAccountXpub {
    pub fn encoded(&self) -> String {
        self.xpub.to_string()
    }
    pub(crate) fn import(encoded: &str) -> Result<Self, WalletError> {
        let xpub = Xpub::from_str(encoded).map_err(|_| WalletError::InvalidWatchOnlyAccount)?;

        if xpub.network != NetworkKind::Test {
            return Err(WalletError::WatchOnlyAccountNetworkMismatch);
        }

        Ok(Self { xpub })
    }
    pub fn receive_address_at(&self, index: u32) -> Result<EthereumReceiveAddress, WalletError> {
        let value = self.derive_receive_address(index)?;

        Ok(EthereumReceiveAddress { index, value })
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

        let serialized = derived.public_key.serialize_uncompressed();

        Ok(eip55_address(&serialized[1..]))
    }
}
const ETHEREUM_METADATA_FORMAT_VERSION: u8 = 1;

/// Versioned public metadata for Ethereum receive-address allocation.
#[derive(Serialize, Deserialize)]
pub struct EthereumReceiveAccountMetadata {
    pub format_version: u8,
    pub account_xpub: String,
    pub network: Network,
    pub next_receive_address_index: u32,
}

/// Allocates unique Ethereum BIP-44 receive addresses.
pub struct EthereumReceiveAccount {
    account: EthereumAccountXpub,
    next_receive_index: u32,
}

impl EthereumReceiveAccount {
    pub fn new(account: EthereumAccountXpub) -> Self {
        Self {
            account,
            next_receive_index: 0,
        }
    }

    pub const fn next_receive_index(&self) -> u32 {
        self.next_receive_index
    }

    pub fn metadata(&self) -> EthereumReceiveAccountMetadata {
        EthereumReceiveAccountMetadata {
            format_version: ETHEREUM_METADATA_FORMAT_VERSION,
            account_xpub: self.account.encoded(),
            network: Network::Testnet,
            next_receive_address_index: self.next_receive_index,
        }
    }

    pub fn restore(metadata: EthereumReceiveAccountMetadata) -> Result<Self, WalletError> {
        if metadata.format_version != ETHEREUM_METADATA_FORMAT_VERSION {
            return Err(WalletError::UnsupportedMetadataVersion);
        }

        if metadata.network != Network::Testnet {
            return Err(WalletError::WalletNetworkMismatch);
        }

        let account = EthereumAccountXpub::import(&metadata.account_xpub)?;

        Ok(Self {
            account,
            next_receive_index: metadata.next_receive_address_index,
        })
    }

    pub fn issue_next_receive_address(&mut self) -> Result<EthereumReceiveAddress, WalletError> {
        let index = self.next_receive_index;

        if index >= (1 << 31) {
            return Err(WalletError::ReceiveAddressIndexExhausted);
        }

        let address = self.account.receive_address_at(index)?;

        self.next_receive_index = index
            .checked_add(1)
            .ok_or(WalletError::ReceiveAddressIndexExhausted)?;

        Ok(address)
    }
}
impl AddressValidator for EthereumAddressValidator {
    fn validate(
        &self,
        asset: AssetId,
        _network: Network,
        address: &str,
    ) -> Result<(), WalletError> {
        if asset.chain != Blockchain::Ethereum
            || !matches!(
                asset.asset,
                Asset::Ethereum | Asset::TetherUsd | Asset::UsdCoin
            )
        {
            return Err(WalletError::InvalidAddress);
        }

        let Some(value) = address.strip_prefix("0x") else {
            return Err(WalletError::InvalidAddress);
        };

        if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(WalletError::InvalidAddress);
        }

        if value == value.to_ascii_lowercase() || value == value.to_ascii_uppercase() {
            return Ok(());
        }

        let lowercase = value.to_ascii_lowercase();
        let hash = Keccak256::digest(lowercase.as_bytes());

        for (index, character) in value.bytes().enumerate() {
            if !character.is_ascii_alphabetic() {
                continue;
            }

            let hash_byte = hash[index / 2];
            let hash_nibble = if index % 2 == 0 {
                hash_byte >> 4
            } else {
                hash_byte & 0x0f
            };

            let should_be_uppercase = hash_nibble >= 8;

            if character.is_ascii_uppercase() != should_be_uppercase {
                return Err(WalletError::InvalidAddress);
            }
        }

        Ok(())
    }
}

/// Derives the public BIP-44 Ethereum account at `m/44'/60'/0'`.
pub fn derive_ethereum_bip44_account(
    seed: &crate::SeedMaterial,
) -> Result<EthereumAccountXpub, WalletError> {
    let purpose =
        ChildNumber::from_hardened_idx(44).map_err(|_| WalletError::KeyDerivationFailed)?;
    let coin_type =
        ChildNumber::from_hardened_idx(60).map_err(|_| WalletError::KeyDerivationFailed)?;
    let account =
        ChildNumber::from_hardened_idx(0).map_err(|_| WalletError::KeyDerivationFailed)?;
    seed.with_bytes(|seed_bytes| {
        let secp = Secp256k1::new();
        let master = Xpriv::new_master(BitcoinNetwork::Testnet, seed_bytes)
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        let derived = master
            .derive_priv(&secp, &[purpose, coin_type, account])
            .map_err(|_| WalletError::KeyDerivationFailed)?;

        Ok(EthereumAccountXpub {
            xpub: Xpub::from_priv(&secp, &derived),
        })
    })
}

/// Derives one checksummed Ethereum BIP-44 receive address.
///
/// Uses the derivation path `m/44'/60'/0'/0/index`.
pub fn derive_ethereum_bip44_receive_address(
    seed: &crate::SeedMaterial,
    index: u32,
) -> Result<String, WalletError> {
    derive_ethereum_bip44_account(seed)?.derive_receive_address(index)
}

fn eip55_address(public_key: &[u8]) -> String {
    let hash = Keccak256::digest(public_key);
    let lowercase: String = hash[12..]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let checksum = Keccak256::digest(lowercase.as_bytes());

    let checksummed: String = lowercase
        .bytes()
        .enumerate()
        .map(|(index, character)| {
            if character.is_ascii_alphabetic() {
                let hash_byte = checksum[index / 2];
                let hash_nibble = if index % 2 == 0 {
                    hash_byte >> 4
                } else {
                    hash_byte & 0x0f
                };

                if hash_nibble >= 8 {
                    return character.to_ascii_uppercase() as char;
                }
            }

            character as char
        })
        .collect();

    format!("0x{checksummed}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecipientAddress;
    use crate::RecoveryPhrase;

    fn ethereum_asset() -> AssetId {
        AssetId::new(Asset::Ethereum, Blockchain::Ethereum).expect("valid Ethereum asset")
    }

    #[test]
    fn accepts_lowercase_ethereum_address() {
        let result = RecipientAddress::parse(
            ethereum_asset(),
            Network::Testnet,
            "0xde709f2102306220921060314715629080e2fb77",
            &EthereumAddressValidator,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn derives_distinct_valid_ethereum_addresses() {
        let phrase = RecoveryPhrase::generate_24().expect("recovery phrase should generate");
        let seed = phrase.derive_seed();

        let first = derive_ethereum_bip44_receive_address(&seed, 0)
            .expect("first Ethereum address should derive");
        let second = derive_ethereum_bip44_receive_address(&seed, 1)
            .expect("second Ethereum address should derive");

        assert!(first.starts_with("0x"));
        assert_eq!(first.len(), 42);
        assert_ne!(first, second);

        let result = EthereumAddressValidator.validate(ethereum_asset(), Network::Testnet, &first);

        assert!(result.is_ok());
    }

    #[test]
    fn accepts_valid_eip55_checksum_address() {
        let result = EthereumAddressValidator.validate(
            ethereum_asset(),
            Network::Testnet,
            "0x52908400098527886E0F7030069857D2E4169EE7",
        );

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_invalid_checksum_address() {
        let result = EthereumAddressValidator.validate(
            ethereum_asset(),
            Network::Testnet,
            "0x52908400098527886E0F7030069857D2E4169Ee7",
        );

        assert_eq!(result, Err(WalletError::InvalidAddress));
    }

    #[test]
    fn rejects_wrong_length_address() {
        let result =
            EthereumAddressValidator.validate(ethereum_asset(), Network::Testnet, "0x1234");

        assert_eq!(result, Err(WalletError::InvalidAddress));
    }
}
