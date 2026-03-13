---
title: Genesis Ceremony
description: How to create a custom genesis configuration for your network
---

The genesis ceremony creates the initial state of an Aztibase network — defining validators, initial balances, and chain parameters.

## CLI Commands

### Initialize

Create a new genesis scaffold:

```bash
aztibase genesis init --output my-network
```

This creates a directory with a default `genesis.toml` template.

### Add Validator

Add a validator to the genesis config:

```bash
aztibase genesis add-validator \
  --genesis my-network/genesis.toml \
  --name validator-1 \
  --address <32-byte-hex-address> \
  --public-key <ed25519-pubkey> \
  --bls-public-key <bls12-381-pubkey> \
  --stake 1000000
```

Or from a keyfile (if you have the validator's JSON):

```bash
aztibase genesis add-validator \
  --genesis my-network/genesis.toml \
  --name validator-1 \
  --key validator.json \
  --stake 1000000
```

### Add Account

Pre-fund an account at genesis:

```bash
aztibase genesis add-account \
  --genesis my-network/genesis.toml \
  --address <address> \
  --balance 10000000
```

### Validate

Check the genesis config for errors:

```bash
aztibase genesis validate --genesis my-network/genesis.toml
```

Validates:
- At least 1 validator exists
- All public keys are valid format
- All stakes are above minimum
- No duplicate addresses
- Total supply is within bounds

### Show

Display the genesis configuration:

```bash
aztibase genesis show --genesis my-network/genesis.toml
```

Shows validators, accounts, genesis hash, and chain parameters.

## Genesis File Format

```toml
[chain]
chain_id = "0xA27B"
timestamp = 1710288000000

[[validators]]
name = "validator-1"
address = "3d2ffd..."
public_key = "ffe4c3..."
bls_public_key = "8d0689..."
stake = 1000000

[[accounts]]
address = "abc123..."
balance = 10000000
```

## Important Notes

- The `genesis generate` subcommand creates a default single-node genesis for quick local testing
- For multi-party networks, use the ceremony flow: `init` → `add-validator` (repeat) → `add-account` (optional) → `validate`
- All participants must use the exact same `genesis.toml` — verify by comparing genesis hashes
- Validator keys are generated separately via `aztibase wallet generate --validator`
