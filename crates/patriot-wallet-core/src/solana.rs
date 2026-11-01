use crate::{AddressValidator, Asset, AssetId, Blockchain, Network, WalletError};
use ed25519_dalek::SigningKey;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha512;
use zeroize::Zeroizing;
/// Validates Solana Base58 public-key addresses.
///

/// A Solana receive address and its hardened BIP-44 account index.
pub struct SolanaReceiveAddress {
    account_index: u32,
    value: String,
}

impl SolanaReceiveAddress {
    pub const fn account_index(&self) -> u32 {
        self.account_index
    }

    pub fn encoded(&self) -> &str {
        &self.value
    }
}

const SOLANA_METADATA_FORMAT_VERSION: u8 = 1;

/// Versioned public metadata for hardened Solana receive accounts.
#[derive(Serialize, Deserialize)]
pub struct SolanaReceiveAccountMetadata {
    pub format_version: u8,
    pub network: Network,
    pub next_account_index: u32,
    pub issued_addresses: Vec<String>,
}

/// Tracks issued Solana public addresses and the next hardened account index.
pub struct SolanaReceiveAccount {
    next_account_index: u32,
    issued_addresses: Vec<String>,
}

impl SolanaReceiveAccount {
    pub fn new() -> Self {
        Self {
            next_account_index: 0,
            issued_addresses: Vec::new(),
        }
    }

    pub const fn next_account_index(&self) -> u32 {
        self.next_account_index
    }

    pub fn metadata(&self) -> SolanaReceiveAccountMetadata {
        SolanaReceiveAccountMetadata {
            format_version: SOLANA_METADATA_FORMAT_VERSION,
            network: Network::Testnet,
            next_account_index: self.next_account_index,
            issued_addresses: self.issued_addresses.clone(),
        }
    }

    pub fn restore(metadata: SolanaReceiveAccountMetadata) -> Result<Self, WalletError> {
        if metadata.format_version != SOLANA_METADATA_FORMAT_VERSION {
            return Err(WalletError::UnsupportedMetadataVersion);
        }

        if metadata.network != Network::Testnet {
            return Err(WalletError::WalletNetworkMismatch);
        }

        let issued_count = u32::try_from(metadata.issued_addresses.len())
            .map_err(|_| WalletError::InvalidStoredMetadata)?;

        if metadata.next_account_index != issued_count
            || metadata.next_account_index > HARDENED_OFFSET
        {
            return Err(WalletError::InvalidStoredMetadata);
        }

        for address in &metadata.issued_addresses {
            validate_solana_public_key(address)?;
        }

        Ok(Self {
            next_account_index: metadata.next_account_index,
            issued_addresses: metadata.issued_addresses,
        })
    }

    pub fn record_next_receive_address(
        &mut self,
        address: String,
    ) -> Result<SolanaReceiveAddress, WalletError> {
        let account_index = self.next_account_index;

        if account_index >= HARDENED_OFFSET {
            return Err(WalletError::ReceiveAddressIndexExhausted);
        }

        validate_solana_public_key(&address)?;

        self.issued_addresses.push(address.clone());
        self.next_account_index = self
            .next_account_index
            .checked_add(1)
            .ok_or(WalletError::ReceiveAddressIndexExhausted)?;

        Ok(SolanaReceiveAddress {
            account_index,
            value: address,
        })
    }
}

pub struct SolanaAddressValidator;

impl AddressValidator for SolanaAddressValidator {
    fn validate(
        &self,
        asset: AssetId,
        _network: Network,
        address: &str,
    ) -> Result<(), WalletError> {
        if asset.chain != Blockchain::Solana
            || !matches!(
                asset.asset,
                Asset::Solana | Asset::TetherUsd | Asset::UsdCoin
            )
        {
            return Err(WalletError::InvalidAddress);
        }

        validate_solana_public_key(address)
    }
}

fn validate_solana_public_key(address: &str) -> Result<(), WalletError> {
    let decoded = bs58::decode(address)
        .into_vec()
        .map_err(|_| WalletError::InvalidAddress)?;

    if decoded.len() != 32 {
        return Err(WalletError::InvalidAddress);
    }

    Ok(())
}

type HmacSha512 = Hmac<Sha512>;

const HARDENED_OFFSET: u32 = 1 << 31;

/// Derives a Solana BIP-44 account address using hardened SLIP-0010 derivation.
///
/// Uses `m/44'/501'/account'/0'`.
pub fn derive_solana_bip44_account_address(
    seed: &crate::SeedMaterial,
    account_index: u32,
) -> Result<String, WalletError> {
    if account_index >= HARDENED_OFFSET {
        return Err(WalletError::KeyDerivationFailed);
    }

    seed.with_bytes(|seed_bytes| {
        let master = hmac_sha512(b"ed25519 seed", seed_bytes)?;

        let mut private_key = Zeroizing::new([0_u8; 32]);
        private_key.copy_from_slice(&master[..32]);

        let mut chain_code = Zeroizing::new([0_u8; 32]);
        chain_code.copy_from_slice(&master[32..]);

        for component in [44_u32, 501, account_index, 0] {
            let hardened_index = component
                .checked_add(HARDENED_OFFSET)
                .ok_or(WalletError::KeyDerivationFailed)?;

            let mut data = Zeroizing::new([0_u8; 37]);
            data[0] = 0;
            data[1..33].copy_from_slice(&private_key[..]);
            data[33..].copy_from_slice(&hardened_index.to_be_bytes());

            let derived = hmac_sha512(&chain_code[..], &data[..])?;
            private_key.copy_from_slice(&derived[..32]);
            chain_code.copy_from_slice(&derived[32..]);
        }

        let signing_key = SigningKey::from_bytes(&private_key);
        let public_key = signing_key.verifying_key();

        Ok(bs58::encode(public_key.as_bytes()).into_string())
    })
}

fn hmac_sha512(key: &[u8], data: &[u8]) -> Result<Zeroizing<[u8; 64]>, WalletError> {
    let mut mac = HmacSha512::new_from_slice(key).map_err(|_| WalletError::KeyDerivationFailed)?;
    mac.update(data);

    let output = mac.finalize().into_bytes();
    let mut bytes = Zeroizing::new([0_u8; 64]);
    bytes.copy_from_slice(&output);

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RecipientAddress, RecoveryPhrase};

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
    fn derives_distinct_valid_solana_account_addresses() {
        let phrase = RecoveryPhrase::generate_24().expect("recovery phrase should generate");
        let seed = phrase.derive_seed();

        let first = derive_solana_bip44_account_address(&seed, 0)
            .expect("first Solana account should derive");
        let second = derive_solana_bip44_account_address(&seed, 1)
            .expect("second Solana account should derive");

        assert_ne!(first, second);
        assert!(
            SolanaAddressValidator
                .validate(solana_asset(), Network::Testnet, &first)
                .is_ok()
        );
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

    #[test]
    fn receive_account_tracks_issued_hardened_addresses() {
        let phrase = RecoveryPhrase::generate_24().expect("recovery phrase should generate");
        let seed = phrase.derive_seed();

        let first_value = derive_solana_bip44_account_address(&seed, 0)
            .expect("first Solana account should derive");
        let second_value = derive_solana_bip44_account_address(&seed, 1)
            .expect("second Solana account should derive");

        let mut account = SolanaReceiveAccount::new();

        let first = account
            .record_next_receive_address(first_value)
            .expect("first address should record");
        let second = account
            .record_next_receive_address(second_value)
            .expect("second address should record");

        assert_eq!(first.account_index(), 0);
        assert_eq!(second.account_index(), 1);
        assert_ne!(first.encoded(), second.encoded());
        assert_eq!(account.next_account_index(), 2);

        let restored =
            SolanaReceiveAccount::restore(account.metadata()).expect("metadata should restore");

        assert_eq!(restored.next_account_index(), 2);
    }
}
