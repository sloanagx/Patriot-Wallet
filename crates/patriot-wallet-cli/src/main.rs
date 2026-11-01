use std::{env, path::PathBuf, process::ExitCode};

use patriot_wallet_core::{
    Asset, AssetId, BitcoinEsploraEndpoint, Blockchain, DogecoinAddressValidator,
    EthereumAddressValidator, MoneroAddressValidator, Network, PersistentBitcoinReceiveAccount,
    PersistentEthereumReceiveAccount, RecipientAddress, SolanaAddressValidator, WalletId,
    create_testnet_ethereum_receive_account, create_testnet_solana_receive_account,
    create_testnet_wallet, issue_next_testnet_solana_receive_address, restore_testnet_wallet,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("Error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().ok_or_else(usage)?;

    match command.as_str() {
        "validate-ethereum-address" => validate_ethereum_address(arguments),
        "validate-dogecoin-address" => validate_dogecoin_address(arguments),
        "validate-monero-address" => validate_monero_address(arguments),
        "new-ethereum-receive-address" => new_ethereum_receive_address(arguments),
        "validate-solana-address" => validate_solana_address(arguments),
        "verify-testnet-wallet" => verify_testnet_wallet(arguments),
        "new-receive-address" => new_receive_address(arguments),
        "create-ethereum-testnet-account" => create_ethereum_testnet_account(arguments),
        "create-solana-testnet-account" => create_solana_testnet_account(arguments),
        "new-solana-receive-address" => new_solana_receive_address(arguments),
        "create-testnet" => create_testnet(arguments),
        "testnet-balance" => testnet_balance(arguments),
        "wallet-info" => wallet_info(arguments),
        _ => Err(usage()),
    }
}

fn create_ethereum_testnet_account(
    mut arguments: impl Iterator<Item = String>,
) -> Result<(), String> {
    let wallet_id = arguments.next().ok_or_else(usage)?;
    let metadata_path = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let wallet_id = WalletId::new(wallet_id).map_err(|error| format!("{error:?}"))?;

    let mut account =
        create_testnet_ethereum_receive_account(wallet_id, PathBuf::from(metadata_path))
            .map_err(|error| format!("{error:?}"))?;

    let address = account
        .issue_next_receive_address()
        .map_err(|error| format!("{error:?}"))?;

    println!("Ethereum testnet receive account created.");
    println!("First receive address: {}", address.encoded());
    println!("Metadata: public watch-only data only.");
    Ok(())
}

fn create_solana_testnet_account(
    mut arguments: impl Iterator<Item = String>,
) -> Result<(), String> {
    let wallet_id = arguments.next().ok_or_else(usage)?;
    let metadata_path = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let wallet_id = WalletId::new(wallet_id).map_err(|error| format!("{error:?}"))?;

    let address = create_testnet_solana_receive_account(wallet_id, PathBuf::from(metadata_path))
        .map_err(|error| format!("{error:?}"))?;

    println!("Solana testnet receive account created.");
    println!("First receive address: {}", address.encoded());
    println!("Account index: {}", address.account_index());
    println!("Metadata: public watch-only data only.");
    Ok(())
}

fn new_solana_receive_address(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let wallet_id = arguments.next().ok_or_else(usage)?;
    let metadata_path = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let wallet_id = WalletId::new(wallet_id).map_err(|error| format!("{error:?}"))?;

    let address =
        issue_next_testnet_solana_receive_address(wallet_id, PathBuf::from(metadata_path))
            .map_err(|error| format!("{error:?}"))?;

    println!("New Solana testnet receive address: {}", address.encoded());
    println!("Account index: {}", address.account_index());
    Ok(())
}

fn create_testnet(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let wallet_id = arguments.next().ok_or_else(usage)?;
    let metadata_path = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let wallet_id = WalletId::new(wallet_id).map_err(|error| format!("{error:?}"))?;

    let mut wallet = create_testnet_wallet(wallet_id, PathBuf::from(metadata_path))
        .map_err(|error| format!("{error:?}"))?;

    let first_address = wallet
        .issue_next_receive_address()
        .map_err(|error| format!("{error:?}"))?;

    println!("Testnet wallet created.");
    println!("Wallet ID: {}", wallet.wallet_id().as_str());
    println!("First receive address: {}", first_address.encoded());
    println!("Recovery phrase: stored in the operating-system credential vault.");
    println!("Metadata: public watch-only data only.");
    Ok(())
}

fn new_receive_address(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let metadata_path = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let mut account = PersistentBitcoinReceiveAccount::load(PathBuf::from(metadata_path))
        .map_err(|error| format!("{error:?}"))?;

    let address = account
        .issue_next_receive_address()
        .map_err(|error| format!("{error:?}"))?;

    println!("New Bitcoin testnet receive address: {}", address.encoded());
    println!("Address index: {}", address.index());
    Ok(())
}

fn new_ethereum_receive_address(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let metadata_path = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let mut account = PersistentEthereumReceiveAccount::load(PathBuf::from(metadata_path))
        .map_err(|error| format!("{error:?}"))?;

    let address = account
        .issue_next_receive_address()
        .map_err(|error| format!("{error:?}"))?;

    println!(
        "New Ethereum testnet receive address: {}",
        address.encoded()
    );
    println!("Address index: {}", address.index());
    Ok(())
}

fn validate_ethereum_address(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let address = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let asset = AssetId::new(Asset::Ethereum, Blockchain::Ethereum)
        .map_err(|error| format!("{error:?}"))?;

    let recipient =
        RecipientAddress::parse(asset, Network::Testnet, address, &EthereumAddressValidator)
            .map_err(|error| format!("{error:?}"))?;

    println!("Valid Ethereum address: {}", recipient.as_str());
    Ok(())
}

fn validate_solana_address(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let address = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let asset =
        AssetId::new(Asset::Solana, Blockchain::Solana).map_err(|error| format!("{error:?}"))?;

    let recipient =
        RecipientAddress::parse(asset, Network::Testnet, address, &SolanaAddressValidator)
            .map_err(|error| format!("{error:?}"))?;

    println!("Valid Solana address: {}", recipient.as_str());
    Ok(())
}

fn validate_dogecoin_address(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let address = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let asset = AssetId::new(Asset::Dogecoin, Blockchain::Dogecoin)
        .map_err(|error| format!("{error:?}"))?;

    let recipient =
        RecipientAddress::parse(asset, Network::Testnet, address, &DogecoinAddressValidator)
            .map_err(|error| format!("{error:?}"))?;

    println!("Valid Dogecoin testnet address: {}", recipient.as_str());
    Ok(())
}

fn validate_monero_address(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let address = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let asset =
        AssetId::new(Asset::Monero, Blockchain::Monero).map_err(|error| format!("{error:?}"))?;

    let recipient =
        RecipientAddress::parse(asset, Network::Testnet, address, &MoneroAddressValidator)
            .map_err(|error| format!("{error:?}"))?;

    println!("Valid Monero testnet address: {}", recipient.as_str());
    Ok(())
}

fn testnet_balance(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let metadata_path = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let account = PersistentBitcoinReceiveAccount::load(PathBuf::from(metadata_path))
        .map_err(|error| format!("{error:?}"))?;

    if account.metadata().network != Network::Testnet {
        return Err("testnet-balance only accepts testnet metadata".to_string());
    }

    let mut watch_only = account
        .watch_only_wallet()
        .map_err(|error| format!("{error:?}"))?;

    let endpoint =
        BitcoinEsploraEndpoint::new(Network::Testnet, "https://mempool.space/testnet/api")
            .map_err(|error| format!("{error:?}"))?;

    watch_only
        .full_scan(&endpoint, 20)
        .map_err(|error| format!("{error:?}"))?;

    println!(
        "Bitcoin testnet balance: {} sats",
        watch_only.balance_sats()
    );
    println!("This result is not stored locally.");
    Ok(())
}

fn verify_testnet_wallet(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let wallet_id = arguments.next().ok_or_else(usage)?;
    let metadata_path = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let wallet_id = WalletId::new(wallet_id).map_err(|error| format!("{error:?}"))?;

    let wallet = restore_testnet_wallet(wallet_id, PathBuf::from(metadata_path))
        .map_err(|error| format!("{error:?}"))?;

    println!("Wallet recovery verification passed.");
    println!("Wallet ID: {}", wallet.wallet_id().as_str());
    println!(
        "Next receive-address index: {}",
        wallet.next_receive_index()
    );
    Ok(())
}

fn wallet_info(mut arguments: impl Iterator<Item = String>) -> Result<(), String> {
    let metadata_path = arguments.next().ok_or_else(usage)?;

    if arguments.next().is_some() {
        return Err(usage());
    }

    let account = PersistentBitcoinReceiveAccount::load(PathBuf::from(metadata_path))
        .map_err(|error| format!("{error:?}"))?;

    let metadata = account.metadata();

    println!("Wallet information");
    println!("Network: {:?}", metadata.network);
    println!("Account public key: {}", metadata.account_xpub);
    println!(
        "Next receive-address index: {}",
        metadata.next_receive_address_index
    );
    println!("This command does not access the recovery phrase.");
    Ok(())
}

fn usage() -> String {
    "Usage:\n  patriot-wallet-cli create-testnet <wallet-id> <metadata-path>\n  patriot-wallet-cli create-ethereum-testnet-account <wallet-id> <metadata-path>\n  patriot-wallet-cli wallet-info <metadata-path>\n  patriot-wallet-cli new-receive-address <metadata-path>\n  patriot-wallet-cli testnet-balance <metadata-path>\n  patriot-wallet-cli validate-ethereum-address <address>\n  patriot-wallet-cli validate-solana-address <address>\n  patriot-wallet-cli validate-dogecoin-address <address>\n  patriot-wallet-cli validate-monero-address <address>\n  patriot-wallet-cli verify-testnet-wallet <wallet-id> <metadata-path>\n patriot-wallet-cli new-ethereum-receive-address <metadata-path>\n patriot-wallet-cli create-solana-testnet-account <wallet-id> <metadata-path>\n patriot-wallet-cli new-solana-receive-address <wallet-id> <metadata-path>"
        .to_string()
}
