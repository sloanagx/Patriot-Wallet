use crate::{AssetId, Network, WalletError};

/// A blockchain-specific validator supplied by a chain integration.
///
/// Implementations must validate the complete address format and network.
pub trait AddressValidator {
    fn validate(&self, asset: AssetId, network: Network, address: &str) -> Result<(), WalletError>;
}

/// A recipient address that has passed chain-specific validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipientAddress {
    asset: AssetId,
    network: Network,
    value: String,
}

impl RecipientAddress {
    pub fn parse(
        asset: AssetId,
        network: Network,
        raw_address: impl Into<String>,
        validator: &impl AddressValidator,
    ) -> Result<Self, WalletError> {
        let value = raw_address.into();

        if value.is_empty() {
            return Err(WalletError::EmptyAddress);
        }

        if value.trim() != value {
            return Err(WalletError::AddressContainsOuterWhitespace);
        }

        validator.validate(asset, network, &value)?;

        Ok(Self {
            asset,
            network,
            value,
        })
    }

    pub const fn asset(&self) -> AssetId {
        self.asset
    }

    pub const fn network(&self) -> Network {
        self.network
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Asset, Blockchain};

    struct ExactAddressValidator;

    impl AddressValidator for ExactAddressValidator {
        fn validate(
            &self,
            _asset: AssetId,
            _network: Network,
            address: &str,
        ) -> Result<(), WalletError> {
            if address == "test-address" {
                Ok(())
            } else {
                Err(WalletError::InvalidAddress)
            }
        }
    }

    fn bitcoin() -> AssetId {
        AssetId::new(Asset::Bitcoin, Blockchain::Bitcoin).expect("valid Bitcoin asset")
    }

    #[test]
    fn validated_address_retains_its_asset_network_and_value() {
        let address = RecipientAddress::parse(
            bitcoin(),
            Network::Testnet,
            "test-address",
            &ExactAddressValidator,
        )
        .expect("address should validate");

        assert_eq!(address.asset(), bitcoin());
        assert_eq!(address.network(), Network::Testnet);
        assert_eq!(address.as_str(), "test-address");
    }

    #[test]
    fn address_with_outer_whitespace_is_rejected() {
        let result = RecipientAddress::parse(
            bitcoin(),
            Network::Testnet,
            " test-address ",
            &ExactAddressValidator,
        );

        assert_eq!(result, Err(WalletError::AddressContainsOuterWhitespace));
    }

    #[test]
    fn invalid_address_is_rejected() {
        let result = RecipientAddress::parse(
            bitcoin(),
            Network::Testnet,
            "wrong-address",
            &ExactAddressValidator,
        );

        assert_eq!(result, Err(WalletError::InvalidAddress));
    }
}
