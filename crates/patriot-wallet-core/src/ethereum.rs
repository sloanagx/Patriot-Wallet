use sha3::{Digest, Keccak256};

use crate::{AddressValidator, Asset, AssetId, Blockchain, Network, WalletError};

/// Validates Ethereum addresses, including EIP-55 mixed-case checksums.
pub struct EthereumAddressValidator;

impl AddressValidator for EthereumAddressValidator {
    fn validate(
        &self,
        asset: AssetId,
        _network: Network,
        address: &str,
    ) -> Result<(), WalletError> {
        if asset.asset != Asset::Ethereum || asset.chain != Blockchain::Ethereum {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecipientAddress;

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
