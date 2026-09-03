use crate::{Asset, BitcoinAmount, Blockchain, Network, RecipientAddress, WalletError};

/// A validated Bitcoin payment proposal.
///
/// This type does not contain UXOs, private keys, signatures, or network code.
pub struct BitcoinSendIntent {
    network: Network,
    recipient: RecipientAddress,
    amount: BitcoinAmount,
}

impl BitcoinSendIntent {
    pub fn new(
        network: Network,
        recipient: RecipientAddress,
        amount: BitcoinAmount,
    ) -> Result<Self, WalletError> {
        if recipient.asset().asset != Asset::Bitcoin
            || recipient.asset().chain != Blockchain::Bitcoin
        {
            return Err(WalletError::RecipientAssetMismatch);
        }

        if recipient.network() != network {
            return Err(WalletError::RecipientNetworkMismatch);
        }

        if amount.sats() == 0 {
            return Err(WalletError::SendAmountMustBePositive);
        }

        Ok(Self {
            network,
            recipient,
            amount,
        })
    }

    pub const fn network(&self) -> Network {
        self.network
    }

    pub fn recipient(&self) -> &RecipientAddress {
        &self.recipient
    }

    pub const fn amount(&self) -> BitcoinAmount {
        self.amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AddressValidator, AssetId, BitcoinAddressValidator, Network, WalletError};

    const VALID_TESTNET_ADDRESS: &str =
        "tb1qrp33g0q5c5txsp9arysrx4k6zdkfs4nce4xj0gdcccefvpysxf3q0sl5k7";

    fn bitcoin_asset() -> AssetId {
        AssetId::new(Asset::Bitcoin, Blockchain::Bitcoin).expect("valid Bitcoin asset")
    }

    fn valid_bitcoin_recipient() -> RecipientAddress {
        RecipientAddress::parse(
            bitcoin_asset(),
            Network::Testnet,
            VALID_TESTNET_ADDRESS,
            &BitcoinAddressValidator,
        )
        .expect("address should validate")
    }

    #[test]
    fn creates_a_testnet_bitcoin_send_intent() {
        let intent = BitcoinSendIntent::new(
            Network::Testnet,
            valid_bitcoin_recipient(),
            BitcoinAmount::from_sats(25_000),
        )
        .expect("intent should be valid");

        assert_eq!(intent.network(), Network::Testnet);
        assert_eq!(intent.amount().sats(), 25_000);
        assert_eq!(intent.recipient().as_str(), VALID_TESTNET_ADDRESS);
    }

    #[test]
    fn rejects_a_zero_value_payment() {
        let result = BitcoinSendIntent::new(
            Network::Testnet,
            valid_bitcoin_recipient(),
            BitcoinAmount::from_sats(0),
        );

        assert!(matches!(result, Err(WalletError::SendAmountMustBePositive)));
    }

    #[test]
    fn rejects_a_recipient_from_the_wrong_network() {
        let result = BitcoinSendIntent::new(
            Network::Mainnet,
            valid_bitcoin_recipient(),
            BitcoinAmount::from_sats(25_000),
        );

        assert!(matches!(result, Err(WalletError::RecipientNetworkMismatch)));
    }

    struct AlwaysValid;

    impl AddressValidator for AlwaysValid {
        fn validate(
            &self,
            _asset: AssetId,
            _network: Network,
            _address: &str,
        ) -> Result<(), WalletError> {
            Ok(())
        }
    }

    #[test]
    fn rejects_a_recipient_for_a_different_asset() {
        let ethereum_asset =
            AssetId::new(Asset::Ethereum, Blockchain::Ethereum).expect("valid Ethereum asset");
        let recipient = RecipientAddress::parse(
            ethereum_asset,
            Network::Testnet,
            "example-address",
            &AlwaysValid,
        )
        .expect("test recipient should validate");

        let result = BitcoinSendIntent::new(
            Network::Testnet,
            recipient,
            BitcoinAmount::from_sats(25_000),
        );

        assert!(matches!(result, Err(WalletError::RecipientAssetMismatch)));
    }
}
