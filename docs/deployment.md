# Deployment Guide

This guide walks through deploying the Quorum governance contracts to Stellar testnet or mainnet.

## Prerequisites

- **Rust** with `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- **Stellar CLI** (`stellar`): install via `cargo install --locked stellar-cli`
- **A funded Stellar account** with at least 100 XLM for contract deployment fees
- **`stellar` login** completed: `stellar network add --name testnet` (or configure mainnet)

## Quick Deploy

```bash
NETWORK=testnet SOURCE_ACCOUNT=G... bash scripts/deploy.sh
```

The script will:
1. Build both contracts to WASM
2. Deploy the QUORUM token contract and mint the initial supply
3. Deploy the governance contract and initialize it with default parameters
4. Print the contract IDs for your `.env.local`

## What the Script Deploys

### 1. QUORUM Token Contract

A Soroban token with:
- **Name:** Quorum
- **Symbol:** QUORUM
- **Decimals:** 7 (matching Stellar's stroop precision)
- **Initial supply:** 100,000,000 QUORUM (100,000,000,000,000 in smallest units)

### 2. Governance Contract

Initialized with the following default parameters:

| Parameter | Default | Meaning |
|-----------|---------|---------|
| `quorum_bps` | 500 | 5% of circulating supply must vote for quorum |
| `voting_period` | 17,280 ledgers | ~24 hours (at ~5s/ledger) |
| `timelock_period` | 8,640 ledgers | ~12 hours |
| `proposal_threshold` | 500,000,000,000 | 50,000 QUORUM minimum to create a proposal |

See [governance-parameters.md](./governance-parameters.md) for guidance on choosing these values.

## Initialization Parameters

The `initialize` call sets all governance parameters **once**. There is no `update_config()` — these values are permanent for the life of the contract instance. Choose carefully.

```
--admin <ADDRESS>          The deployer address (can cancel proposals)
--token <TOKEN_ID>         The QUORUM token contract address
--quorum_bps <U32>         Quorum threshold in basis points (e.g. 500 = 5%)
--voting_period <U32>      Voting window in ledgers (17,280 ≈ 1 day)
--timelock_period <U32>    Timelock delay in ledgers (8,640 ≈ 12 hours)
--proposal_threshold <I128> Minimum token balance to create a proposal (in smallest units)
```

## Post-Deployment Verification

After deployment, verify the contracts are working:

```bash
# Check governance config
stellar contract invoke --id $GOV_ID --rpc-url $RPC_URL \
  --network-passphrase "$PASSPHRASE" -- get_config

# Check token supply
stellar contract invoke --id $TOKEN_ID --rpc-url $RPC_URL \
  --network-passphrase "$PASSPHRASE" -- total_supply

# Verify token balance of deployer
stellar contract invoke --id $TOKEN_ID --rpc-url $RPC_URL \
  --network-passphrase "$PASSPHRASE" -- balance --owner $SOURCE_ACCOUNT
```

## Environment Variables

Add these to your `.env.local` for the frontend:

```bash
NEXT_PUBLIC_GOVERNANCE_CONTRACT=<GOV_ID>
NEXT_PUBLIC_TOKEN_CONTRACT=<TOKEN_ID>
NEXT_PUBLIC_STELLAR_NETWORK=testnet
```

## Troubleshooting

- **"Already initialized"**: The contract was already initialized. Each instance is single-use — deploy a new contract to re-initialize.
- **Insufficient balance**: Fund your account with XLM via the [Stellar Laboratory](https://laboratory.stellar.org/) faucet.
- **RPC errors**: Ensure your RPC URL matches the network (testnet vs mainnet).
