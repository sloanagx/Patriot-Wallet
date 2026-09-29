use bitcoin::hashes::{Hash, sha256d};

use crate::{AddressValidator, Asset, AssetId, Blockchain, Network, WalletError};

/// Validates Dogecoin P2PKH and P2SH Base58Check recipient addresses.
pub struct DogecoinAddressValidator;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecipientAddress;

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
}
