# Patriot Wallet Security Rules

- Private keys, seed phrases, and wallet passwords must never be logged.
- No cryptographic primitives or address parsers may be implemented from scratch.
- Every recipient address must be validated by the implementation for its specific blockchain before a transaction can be created or signed.
- An asset is valid only on the blockchain explicitly recorded in its AssetId.
- Mainnet transactions must require an explicit user confirmation in the future GUI.
- Development uses testnets only. No real funds during development.