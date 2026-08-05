use bitcoin::{Address as BitcoinAddress, Network as BitcoinNetwork, address::NetworkUnchecked};

use crate::{AddressValidator, Asset, AssetId, Blockchain, Network, WalletError};

/// Validates Bitcoin addresses using the maintained `bitcoin` crate.
pub struct BitcoinAddressValidator;

impl AddressValidator for BitcoinAddressValidator {
    fn validate(&self, asset: AssetId, network: Network, address: &str) -> Result<(), WalletError> {
        if asset.asset != Asset::Bitcoin || asset.chain != Blockchain::Bitcoin {
            return Err(WalletError::InvalidAddress);
        }

        let expected_network = match network {
            Network::Mainnet => BitcoinNetwork::Bitcoin,
            Network::Testnet => BitcoinNetwork::Testnet,
        };

        let parsed = address
            .parse::<BitcoinAddress<NetworkUnchecked>>()
            .map_err(|_| WalletError::InvalidAddress)?;

        parsed
            .require_network(expected_network)
            .map_err(|_| WalletError::AddressNetworkMismatch)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TESTNET_ADDRESS: &str =
        "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7";

    fn bitcoin_asset() -> AssetId {
        AssetId::new(Asset::Bitcoin, Blockchain::Bitcoin).expect("valid Bitcoin asset")
    }

    #[test]
    fn accepts_a_valid_bitcoin_testnet_address() {
        let result = BitcoinAddressValidator.validate(
            bitcoin_asset(),
            Network::Testnet,
            VALID_TESTNET_ADDRESS,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_a_testnet_address_when_mainnet_is_selected() {
        let result = BitcoinAddressValidator.validate(
            bitcoin_asset(),
            Network::Mainnet,
            VALID_TESTNET_ADDRESS,
        );

        assert_eq!(result, Err(WalletError::AddressNetworkMismatch));
    }

    #[test]
    fn rejects_malformed_bitcoin_addresses() {
        let result = BitcoinAddressValidator.validate(
            bitcoin_asset(),
            Network::Testnet,
            "not-a-bitcoin-address",
        );

        assert_eq!(result, Err(WalletError::InvalidAddress));
    }
}
