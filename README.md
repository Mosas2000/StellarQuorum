# Quorum

Stellar Quorum is an open-source governance infrastructure layer for the Stellar and Soroban ecosystem. It provides the core primitives for decentralized, token-weighted decision-making: proposal creation, on-chain voting, delegation, timelock execution, and transparent result reporting.

As Stellar moves from a foundation-driven upgrade model to community-driven governance, the ecosystem needs reliable, auditable governance tooling that DeFi protocols, DAOs and community organizations can adopt without building from scratch.

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
await client.getVote(1n, 'G...');       // How they voted: 0=Against, 1=For, 2=Abstain, null=not voted

// Build transactions (returns unsigned XDR for Freighter signing)
await client.buildCreateProposal(address, title, description);
await client.buildVote(voter, proposalId, support); // support: 0=Against, 1=For, 2=Abstain
await client.buildFinalize(proposalId);
await client.buildExecute(proposalId);
```
