# Glossary of Governance Terms

## Stellar / Soroban Concepts

### Ledger
A unit of time on the Stellar network. Each ledger closes approximately every **5 seconds**. All on-chain timestamps (voting periods, timelocks, snapshots) are measured in ledger sequence numbers, not wall-clock time. To convert: `ledgers ≈ seconds / 5`. For example, 17,280 ledgers ≈ 1 day.

### Snapshot Ledger
The ledger sequence at which a proposal is created. Token balances at this ledger determine each holder's voting power for that proposal. This prevents flash-loan manipulation — tokens borrowed after the snapshot carry no weight.

### Soroban
Stellar's smart contract platform. Quorum's governance and token contracts are Soroban programs deployed to the Stellar network.

### Persistent Storage
Soroban storage that survives across ledger closes (unlike instance storage). Proposals and vote records use persistent storage so they remain readable for the duration of the voting window and beyond.

### TTL (Time-to-Live)
How long a Soroban storage entry remains readable before it expires. Quorum bumps persistent entries to 90-day TTLs so long-lived proposals don't vanish mid-vote.

---

## Governance Concepts

### Quorum
The minimum total voting power (For + Against + Abstain) required for a proposal to be eligible for execution. Expressed in basis points (BPS) of circulating token supply. Example: `quorum_bps = 500` means 5% of total supply. Without reaching quorum, a proposal fails regardless of vote distribution.

### Basis Points (BPS)
One hundredth of a percent. 1 BPS = 0.01%. Used to express the quorum threshold. `500 BPS = 5%`, `1000 BPS = 10%`.

### Voting Power
The weight a token holder's vote carries. Derived from their QUORUM token balance at the proposal's snapshot ledger, not their current live balance.

### Voting Period
The window during which votes are accepted, measured in ledgers. At ~5 seconds per ledger, `17,280 ledgers ≈ 1 day`. A voting period of `17,280` means votes are accepted for approximately 24 hours.

### Proposal Threshold
The minimum token balance a proposer must hold to create a proposal. Measured in the token's smallest unit (stroops, with 7 decimals). Example: `500000000000` = 50,000 QUORUM. This prevents spam.

### Timelock
A mandatory waiting period between a proposal passing and its execution. During the timelock, a guardian multisig can veto. Measured in ledgers. `8,640 ledgers ≈ 12 hours`.

### Finalize
The act of closing a proposal's voting window and determining its outcome. A proposal passes if quorum is reached AND the For votes exceed the Against votes. Anyone can call `finalize` after the voting period ends.

### Execute
The act of triggering a queued proposal's on-chain actions after the timelock expires. Only possible for proposals in `Queued` status.

### Guardian
A multisig address that can veto proposals during the timelock window. Acts as a safety net against governance attacks.

### Delegation
(Planned) The ability to assign your voting power to another address without transferring token ownership.

---

## Proposal Lifecycle

| State | Description |
|-------|-------------|
| **Pending** | Created, voting not yet started |
| **Active** | Voting window is open |
| **Failed** | Quorum not reached or majority Against |
| **Queued** | Passed, waiting in timelock |
| **Executed** | Timelock expired, actions executed |
| **Cancelled** | Cancelled by proposer or admin before execution |

---

## First-Use Links

- The [deployment guide](./deployment.md) explains how to deploy and initialize the contracts.
- The [governance parameters guide](./governance-parameters.md) explains how to choose values for `quorum_bps`, `voting_period`, `timelock_period`, and `proposal_threshold`.
- The [security model](./security-model.md) describes threat assumptions and trust boundaries.
