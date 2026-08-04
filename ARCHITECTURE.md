# Patriot Wallet Architecture

## Core

`patriot-wallet-core` owns the security-sensitive domain model:

- supported assets and blockchains
- network selection
- address validation interfaces
- transaction intent and review
- key-vault interfaces

It must not contain GUI code or directly expose secrets.

## Integrations

Each blockchain will have its own isolated integration module:

- Bitcoin
- Ethereum and ERC-20 tokens (USDT, USDC)
- Solana and SPL tokens (USDT, USDC)
- Dogecoin
- Monero

Each module validates addresses and builds unsigned transactions according to that chain's official protocol rules.

## User interface

The GUI will be a separate Tauri application added after the core, CLI test flows, and testnet transaction flows are complete.