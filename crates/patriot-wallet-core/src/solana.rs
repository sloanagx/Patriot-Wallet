use crate::{AddressValidator, Asset, AssetId, Blockchain, Network, WalletError};

/// Validates Solana Base58 public-key addresses.
pub struct SolanaAddressValidator;

impl AddressValidator for SolanaAddressValidator {
    fn validate(
        &self,
        asset: AssetId,
        _network: Network,
        address: &str,
    ) -> Result<(), WalletError> {
        if asset.asset != Asset::Solana || asset.chain != Blockchain::Solana {
            return Err(WalletError::InvalidAddress);
        }

        let decoded = bs58::decode(address)
            .into_vec()
            .map_err(|_| WalletError::InvalidAddress)?;

        if decoded.len() != 32 {
            return Err(WalletError::InvalidAddress);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecipientAddress;

    const VALID_SOLANA_ADDRESS: &str = "So11111111111111111111111111111111111111112";

    fn solana_asset() -> AssetId {
        AssetId::new(Asset::Solana, Blockchain::Solana).expect("valid Solana asset")
    }

    #[test]
    fn accepts_a_valid_solana_address() {
        let result = RecipientAddress::parse(
            solana_asset(),
            Network::Testnet,
            VALID_SOLANA_ADDRESS,
            &SolanaAddressValidator,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_non_base58_input() {
        let result = SolanaAddressValidator.validate(
            solana_asset(),
            Network::Testnet,
            "not-a-valid-solana-address-0",
        );

        assert_eq!(result, Err(WalletError::InvalidAddress));
    }

    #[test]
    fn rejects_a_base58_value_with_the_wrong_length() {
        let too_short = "1".repeat(31);

        let result = SolanaAddressValidator.validate(solana_asset(), Network::Testnet, &too_short);

        assert_eq!(result, Err(WalletError::InvalidAddress));
    }
}
