use sha3::{Digest, Keccak256};

use crate::{AddressValidator, Asset, AssetId, Blockchain, Network, WalletError};

const MAINNET_PRIMARY_PREFIX: u8 = 18;
const MAINNET_SUBADDRESS_PREFIX: u8 = 42;
const TESTNET_PRIMARY_PREFIX: u8 = 53;
const TESTNET_SUBADDRESS_PREFIX: u8 = 63;

const STANDARD_ADDRESS_BYTES: usize = 69;
const CHECKSUM_BYTES: usize = 4;

/// Validates Monero primary addresses and subaddresses.
pub struct MoneroAddressValidator;

impl AddressValidator for MoneroAddressValidator {
    fn validate(&self, asset: AssetId, network: Network, address: &str) -> Result<(), WalletError> {
        if asset.asset != Asset::Monero || asset.chain != Blockchain::Monero {
            return Err(WalletError::InvalidAddress);
        }

        let decoded = decode_monero_base58(address)?;

        if decoded.len() != STANDARD_ADDRESS_BYTES {
            return Err(WalletError::InvalidAddress);
        }

        let checksum_start = STANDARD_ADDRESS_BYTES - CHECKSUM_BYTES;
        let expected_checksum = Keccak256::digest(&decoded[..checksum_start]);

        if decoded[checksum_start..] != expected_checksum[..CHECKSUM_BYTES] {
            return Err(WalletError::InvalidAddress);
        }

        let valid_prefix = match network {
            Network::Mainnet => matches!(
                decoded[0],
                MAINNET_PRIMARY_PREFIX | MAINNET_SUBADDRESS_PREFIX
            ),
            Network::Testnet => matches!(
                decoded[0],
                TESTNET_PRIMARY_PREFIX | TESTNET_SUBADDRESS_PREFIX
            ),
        };

        if !valid_prefix {
            return Err(WalletError::AddressNetworkMismatch);
        }

        Ok(())
    }
}

const MONERO_BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

fn decode_monero_base58(address: &str) -> Result<Vec<u8>, WalletError> {
    if address.is_empty() {
        return Err(WalletError::InvalidAddress);
    }

    let full_block_count = address.len() / 11;
    let trailing_characters = address.len() % 11;
    let mut decoded = Vec::new();

    for block_index in 0..full_block_count {
        let start = block_index * 11;
        let end = start + 11;

        decode_monero_block(&address.as_bytes()[start..end], 8, &mut decoded)?;
    }

    if trailing_characters == 1 {
        return Err(WalletError::InvalidAddress);
    }

    if trailing_characters > 0 {
        let trailing_bytes = match trailing_characters {
            2 => 1,
            3 => 2,
            5 => 3,
            6 => 4,
            7 => 5,
            9 => 6,
            10 => 7,
            _ => return Err(WalletError::InvalidAddress),
        };

        let start = full_block_count * 11;
        decode_monero_block(&address.as_bytes()[start..], trailing_bytes, &mut decoded)?;
    }

    Ok(decoded)
}

fn decode_monero_block(
    block: &[u8],
    output_bytes: usize,
    decoded: &mut Vec<u8>,
) -> Result<(), WalletError> {
    let mut value = 0_u64;

    for character in block {
        let digit = MONERO_BASE58
            .iter()
            .position(|candidate| candidate == character)
            .ok_or(WalletError::InvalidAddress)? as u64;

        value = value
            .checked_mul(58)
            .and_then(|current| current.checked_add(digit))
            .ok_or(WalletError::InvalidAddress)?;
    }

    if output_bytes < 8 && value >= (1_u64 << (output_bytes * 8)) {
        return Err(WalletError::InvalidAddress);
    }

    let bytes = value.to_be_bytes();
    decoded.extend_from_slice(&bytes[8 - output_bytes..]);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RecipientAddress;

    fn monero_asset() -> AssetId {
        AssetId::new(Asset::Monero, Blockchain::Monero).expect("valid Monero asset")
    }

    fn address_with_prefix(prefix: u8) -> String {
        let mut bytes = vec![prefix];
        bytes.extend_from_slice(&[9_u8; 64]);

        let checksum = Keccak256::digest(&bytes);
        bytes.extend_from_slice(&checksum[..CHECKSUM_BYTES]);

        encode_monero_base58(&bytes)
    }

    fn encode_monero_base58(bytes: &[u8]) -> String {
        bytes.chunks(8).map(encode_monero_block).collect()
    }

    fn encode_monero_block(block: &[u8]) -> String {
        let output_characters = match block.len() {
            1 => 2,
            2 => 3,
            3 => 5,
            4 => 6,
            5 => 7,
            6 => 9,
            7 => 10,
            8 => 11,
            _ => unreachable!("Monero Base58 blocks are never empty"),
        };

        let value = block
            .iter()
            .fold(0_u64, |current, byte| (current << 8) | u64::from(*byte));

        let mut value = value;
        let mut output = vec![b'1'; output_characters];

        for character in output.iter_mut().rev() {
            *character = MONERO_BASE58[(value % 58) as usize];
            value /= 58;
        }

        String::from_utf8(output).expect("Base58 alphabet is valid UTF-8")
    }

    #[test]
    fn accepts_a_valid_mainnet_primary_address() {
        let address = address_with_prefix(MAINNET_PRIMARY_PREFIX);

        assert_eq!(address.len(), 95);

        let result = RecipientAddress::parse(
            monero_asset(),
            Network::Mainnet,
            address,
            &MoneroAddressValidator,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn rejects_a_testnet_address_when_mainnet_is_selected() {
        let address = address_with_prefix(TESTNET_PRIMARY_PREFIX);

        let result = MoneroAddressValidator.validate(monero_asset(), Network::Mainnet, &address);

        assert_eq!(result, Err(WalletError::AddressNetworkMismatch));
    }

    #[test]
    fn rejects_an_address_with_an_invalid_checksum() {
        let mut address = address_with_prefix(MAINNET_PRIMARY_PREFIX);
        let replacement = if address.ends_with('1') { '2' } else { '1' };
        address.pop();
        address.push(replacement);

        let result = MoneroAddressValidator.validate(monero_asset(), Network::Mainnet, &address);

        assert_eq!(result, Err(WalletError::InvalidAddress));
    }
}
