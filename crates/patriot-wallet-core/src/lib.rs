//! Security-focused, chain-agnostic wallet domain logic.
//!
//! This crate must never log, expose, or silently transmit secret material.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

mod account;
mod address;
mod bdk;
mod bitcoin;
mod dogecoin;
mod dogecoin_storage;
mod ethereum;
mod ethereum_storage;
mod hd;
mod mnemonic;
mod money;
mod send;
mod setup;
mod solana;
mod solana_storage;
mod storage;
mod vault;

pub use account::{BitcoinReceiveAccount, BitcoinReceiveAccountMetadata};
pub use address::{AddressValidator, RecipientAddress};
pub use bdk::{BdkBitcoinWatchOnlyWallet, BitcoinEsploraEndpoint};
pub use bitcoin::BitcoinAddressValidator;
pub use dogecoin::{
    DogecoinAccountXpub, DogecoinAddressValidator, DogecoinReceiveAccount,
    DogecoinReceiveAccountMetadata, DogecoinReceiveAddress, derive_dogecoin_bip44_account,
};
pub use dogecoin_storage::PersistentDogecoinReceiveAccount;
pub use mnemonic::{RecoveryPhrase, SeedMaterial};
pub use money::BitcoinAmount;
pub const PRODUCT_NAME: &str = "Patriot Wallet";
pub use ethereum::{
    EthereumAccountXpub, EthereumAddressValidator, EthereumReceiveAccount,
    EthereumReceiveAccountMetadata, EthereumReceiveAddress, derive_ethereum_bip44_account,
    derive_ethereum_bip44_receive_address,
};
pub use ethereum_storage::PersistentEthereumReceiveAccount;
pub use hd::{BitcoinAccountXpub, BitcoinReceiveAddress, derive_bitcoin_bip84_account};
pub use send::BitcoinSendIntent;
pub use setup::{
    TestnetWallet, create_testnet_dogecoin_receive_account,
    create_testnet_ethereum_receive_account, create_testnet_solana_receive_account,
    create_testnet_wallet, issue_next_testnet_solana_receive_address, restore_testnet_wallet,
};
pub use solana::{
    SolanaAddressValidator, SolanaReceiveAccount, SolanaReceiveAccountMetadata,
    SolanaReceiveAddress, derive_solana_bip44_account_address,
};
pub use solana_storage::PersistentSolanaReceiveAccount;
pub use storage::PersistentBitcoinReceiveAccount;
pub use vault::{OsKeychainRecoveryVault, RecoveryPhraseVault, WalletId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
}

impl Network {
    pub const fn is_mainnet(self) -> bool {
        matches!(self, Self::Mainnet)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Blockchain {
    Bitcoin,
    Ethereum,
    Solana,
    Dogecoin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Asset {
    Bitcoin,
    Ethereum,
    Solana,
    Dogecoin,
    TetherUsd,
    UsdCoin,
}

impl Asset {
    pub const fn ticker(self) -> &'static str {
        match self {
            Self::Bitcoin => "BTC",
            Self::Ethereum => "ETH",
            Self::Solana => "SOL",
            Self::Dogecoin => "DOGE",
            Self::TetherUsd => "USDT",
            Self::UsdCoin => "USDC",
        }
    }

    pub const fn is_supported_on(self, chain: Blockchain) -> bool {
        matches!(
            (self, chain),
            (Self::Bitcoin, Blockchain::Bitcoin)
                | (Self::Ethereum, Blockchain::Ethereum)
                | (Self::Solana, Blockchain::Solana)
                | (Self::Dogecoin, Blockchain::Dogecoin)
                | (Self::TetherUsd, Blockchain::Ethereum | Blockchain::Solana)
                | (Self::UsdCoin, Blockchain::Ethereum | Blockchain::Solana)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetId {
    pub asset: Asset,
    pub chain: Blockchain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletError {
    UnsupportedAssetOnChain { asset: Asset, chain: Blockchain },
    EmptyAddress,
    AddressContainsOuterWhitespace,
    InvalidAddress,
    AddressNetworkMismatch,
    InvalidMnemonic,
    MnemonicGenerationFailed,
    KeyDerivationFailed,
    ReceiveAddressIndexExhausted,
    InvalidWatchOnlyAccount,
    WatchOnlyAccountNetworkMismatch,
    UnsupportedMetadataVersion,
    MetadataSerializationFailed,
    MetadataDeserializationFailed,
    MetadataStorageFailed,
    InvalidStoredMetadata,
    AmountOverflow,
    InsufficientFunds,
    RecipientAssetMismatch,
    RecipientNetworkMismatch,
    SendAmountMustBePositive,
    BdkWalletCreationFailed,
    InvalidEsploraEndpoint,
    EsploraNetworkMismatch,
    InvalidSyncConfiguration,
    EsploraSyncFailed,
    InvalidWalletId,
    KeychainAccessFailed,
    RecoveryPhraseNotFound,
    InvalidStoredRecoveryPhrase,
    WalletAlreadyExists,
    MetadataAlreadyExists,
    WalletNetworkMismatch,
    RecoveryPhraseDoesNotMatchMetadata,
}

impl AssetId {
    pub const fn new(asset: Asset, chain: Blockchain) -> Result<Self, WalletError> {
        if asset.is_supported_on(chain) {
            Ok(Self { asset, chain })
        } else {
            Err(WalletError::UnsupportedAssetOnChain { asset, chain })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mainnet_is_identified_correctly() {
        assert!(Network::Mainnet.is_mainnet());
        assert!(!Network::Testnet.is_mainnet());
    }

    #[test]
    fn every_requested_asset_has_its_expected_ticker() {
        assert_eq!(Asset::Bitcoin.ticker(), "BTC");
        assert_eq!(Asset::Ethereum.ticker(), "ETH");
        assert_eq!(Asset::Solana.ticker(), "SOL");
        assert_eq!(Asset::Dogecoin.ticker(), "DOGE");
        assert_eq!(Asset::TetherUsd.ticker(), "USDT");
        assert_eq!(Asset::UsdCoin.ticker(), "USDC");
    }

    #[test]
    fn usdt_is_supported_on_ethereum_and_solana() {
        assert!(AssetId::new(Asset::TetherUsd, Blockchain::Ethereum).is_ok());
        assert!(AssetId::new(Asset::TetherUsd, Blockchain::Solana).is_ok());
    }

    #[test]
    fn bitcoin_cannot_be_created_on_ethereum() {
        let result = AssetId::new(Asset::Bitcoin, Blockchain::Ethereum);

        assert_eq!(
            result,
            Err(WalletError::UnsupportedAssetOnChain {
                asset: Asset::Bitcoin,
                chain: Blockchain::Ethereum,
            })
        );
    }
}
