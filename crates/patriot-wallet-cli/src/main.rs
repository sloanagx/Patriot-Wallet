use std::{env, path::PathBuf, process::ExitCode};

use patriot_wallet_core::{WalletId, create_testnet_wallet};

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
    if command != "create-testnet" {
        return Err(usage());
    }

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

    println!("Tesnet wallet created.");
    println!("Wallet ID: {}", wallet.wallet_id().as_str());
    println!("First receive address: {}", first_address.encoded());
    println!("Recovery phrase: stored in the operating-system credential vault.");
    println!("Metadata: public watch-only data only.");
    Ok(())
}

fn usage() -> String {
    "Usage: patriot-wallet-cli create-testnet <wallet-id> <metadata-path>".to_string()
}
