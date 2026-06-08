# Quorum

> On-chain governance infrastructure for the Stellar and Soroban ecosystem.

![License](https://img.shields.io/badge/license-MIT-blue)
![Stellar Ecosystem](https://img.shields.io/badge/ecosystem-Stellar-blue)
![Drips Wave](https://img.shields.io/badge/Drips-Wave%20Contributor-green)
![TypeScript](https://img.shields.io/badge/language-TypeScript-blue)
![Rust](https://img.shields.io/badge/language-Rust-orange)

---

## Overview

Quorum is an open-source governance infrastructure layer for the Stellar and Soroban ecosystem. It provides the core primitives for decentralized, token-weighted decision-making: proposal creation, on-chain voting, delegation, timelock execution, and transparent result reporting.

As Stellar moves from a foundation-driven upgrade model to community-driven governance — a stated priority in the SDF's 2026 roadmap — the ecosystem needs reliable, auditable governance tooling that DeFi protocols, DAOs, and community organizations can adopt without building from scratch.

---

## Repository Structure

```
quorum/
├── frontend/              # Next.js governance UI
│   ├── app/
│   │   ├── page.tsx       # Dashboard with live stats and recent proposals
│   │   ├── proposals/     # Proposals list + detail pages
│   │   └── create/        # Create proposal form
│   ├── components/
│   │   ├── Navbar.tsx
│   │   ├── Footer.tsx
│   │   ├── ProposalCard.tsx
│   │   ├── VoteBar.tsx
│   │   └── StatusBadge.tsx
│   └── lib/
│       ├── types.ts        # Proposal, Vote, ProposalStatus interfaces
│       └── proposals.ts    # Mock proposal data + getProposalById()
│
├── contracts/             # Soroban smart contracts (Rust)
│   ├── governance/        # Proposal creation, voting, timelock, execution
│   │   └── src/lib.rs
│   ├── token/             # QUORUM governance token (SEP-41 compatible)
│   │   └── src/lib.rs
│   └── Cargo.toml         # Workspace
│
├── sdk/                   # TypeScript SDK for dApp integration
│   ├── src/
│   │   ├── client.ts      # QuorumClient — typed proposal/vote methods
│   │   ├── types.ts       # Shared TypeScript types
│   │   └── index.ts
│   └── package.json
│
├── scripts/
│   └── deploy.sh          # Deploy contracts to testnet/mainnet
│
├── .github/
│   └── workflows/ci.yml   # CI: contracts + frontend + SDK
│
└── README.md
```

---

## Governance Model

### Proposal Lifecycle

| State | Description |
|---|---|
| Pending | Created, voting not yet started |
| Active | Voting window is open |
| Failed | Quorum not reached or majority Against |
| Queued | Passed, waiting in 48-hour timelock |
| Executed | Timelock expired, on-chain actions executed |
| Cancelled | Cancelled by proposer before voting ends |

### Quorum

The quorum threshold is the minimum total voting power (For + Against + Abstain) required for a proposal to be eligible for execution. Without quorum, a proposal fails regardless of vote distribution. Default: 5% of circulating QUORUM supply.

### Voting Power

Voting power is derived from QUORUM token balance at the snapshot ledger taken at proposal creation. This prevents flash-loan manipulation of governance votes.

### Timelock

All passed proposals enter a 48-hour timelock before execution. A guardian multisig can veto during this window as a safety net against governance attacks.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Frontend | Next.js 16, TypeScript, Tailwind CSS v4 |
| Governance Contract | Soroban (Rust), soroban-sdk v22 |
| Token Contract | Soroban (Rust), SEP-41 compatible |
| SDK | TypeScript, @stellar/stellar-sdk |
| Wallet | Freighter (planned) |
| CI | GitHub Actions |

---

## Getting Started

### Run the Frontend

```bash
git clone https://github.com/StellarQuorum/StellarQuorum.git
cd StellarQuorum/frontend
npm install && npm run dev
```

Open [http://localhost:3000](http://localhost:3000)

### Build the Contracts

```bash
cd contracts
cargo build --target wasm32-unknown-unknown --release
```

### Deploy to Testnet

```bash
SOURCE_ACCOUNT=G... NETWORK=testnet bash scripts/deploy.sh
```

### Use the SDK

```typescript
import { QuorumClient, TESTNET } from '@quorum/sdk';

const client = new QuorumClient({
  ...TESTNET,
  governanceContractId: 'CC...',
  tokenContractId: 'CC...',
});

const proposals = await client.getAllProposals();
const xdr = await client.buildVote(voterAddress, 1n, 1); // Vote For on proposal 1
```

---

## Roadmap

- [ ] Freighter wallet integration for live voting on Stellar testnet
- [ ] Soroban governance contract testnet deployment
- [ ] Token delegation UI — delegate voting power without transferring tokens
- [ ] Timelock execution engine — automated execution after 48h delay
- [ ] Multi-sig proposal creation
- [ ] Governor contract security audit
- [ ] QUORUM token distribution and staking
- [ ] Off-chain signaling (Snapshot-style) before on-chain execution
- [ ] Governance analytics dashboard

---

## Drips Wave

Quorum participates in the **Stellar Wave** on [Drips Network](https://www.drips.network/wave).

Rewarded issues use `drips:*` labels to show the expected points value:

| Label | Scope |
|---|---|
| `drips:1` | Small tasks such as docs, styling, and good first issues |
| `drips:3` | Medium tasks such as new components or focused test coverage |
| `drips:5` | Large tasks such as new pages or contract functions |
| `drips:8` | Complex tasks such as full features or security-sensitive work |

See [docs/contributing.md](docs/contributing.md#drips-wave) for the contributor
claim flow and maintainer setup checklist.

**Good first issues:**
- Build the Freighter wallet connection component
- Write the Soroban governance contract `cancel()` function
- Add proposal search and sorting to the proposals list
- Implement delegation UI
- Write unit tests for the proposal data layer

---

## Contributing

1. Fork the repo and create a feature branch
2. Make your changes with clear commit messages
3. Open a PR referencing the issue

---

## License

MIT — free to use, modify, and distribute.

---

## SDK Reference

### `QuorumClient`

```typescript
import { QuorumClient, TESTNET } from '@quorum/sdk';

const client = new QuorumClient({ ...TESTNET, governanceContractId: 'CC...', tokenContractId: 'CC...' });

// Read proposals
await client.getProposal(1n);           // Get proposal by ID
await client.getAllProposals();          // Get all proposals
await client.getProposalCount();        // Total proposal count
await client.getConfig();               // Protocol config (quorum, voting period, etc.)
await client.hasVoted(1n, 'G...');      // Check if address voted

// Build transactions (returns unsigned XDR for Freighter signing)
await client.buildCreateProposal(address, title, description);
await client.buildVote(voter, proposalId, support); // support: 0=Against, 1=For, 2=Abstain
await client.buildFinalize(proposalId);
await client.buildExecute(proposalId);
```
