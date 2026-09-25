# Security Model & Threat Assumptions

## Overview

Quorum is a token-weighted governance system on Stellar. This document describes the threats it is designed to resist, the trust assumptions it makes, and the known gaps.

## Threat Model

### 1. Flash-Loan Vote Manipulation
**Threat:** An attacker borrows a large amount of QUORUM tokens, votes, and returns them in a single transaction.
**Mitigation:** Voting power is read at the proposal's **snapshot ledger** (the ledger when the proposal was created), not the current balance. Tokens acquired after the snapshot carry zero voting power. The snapshot is immutable once set.

### 2. Governance Attack via Low Quorum
**Threat:** An attacker with a small token fraction passes a proposal when few others vote.
**Mitigation:** The `quorum_bps` parameter sets a minimum total voting power (For + Against + Abstain) that must participate. Without reaching quorum, a proposal fails regardless of the vote distribution.

### 3. Rush Execution
**Threat:** A passed proposal is executed immediately before token holders can react.
**Mitigation:** All passed proposals enter a **timelock** (default ~12 hours) before execution. During this window, the guardian multisig can veto.

### 4. Spam Proposals
**Threat:** An attacker fills the proposal queue with low-quality proposals.
**Mitigation:** The `proposal_threshold` requires proposers to hold a minimum token balance (default 50,000 QUORUM). This makes spamming costly.

### 5. Admin Key Compromise
**Threat:** The admin key is stolen or misused.
**Mitigation:** The admin can only cancel proposals — they cannot vote on behalf of others, execute without timelock, or change governance parameters. The admin key should be held in a multisig or hardware wallet.

### 6. Guardian Abuse
**Threat:** The guardian multisig vetoes a legitimate proposal.
**Mitigation:** The guardian is a trusted role (typically a security multisig). This is a deliberate trade-off: the guardian provides a safety net against governance attacks at the cost of centralized veto power. The guardian cannot execute or modify proposals — only veto during the timelock window.

---

## Trust Assumptions

| Actor | Power | Assumption |
|-------|-------|------------|
| **Admin** | Cancel any proposal | Holds the admin key; acts in good faith or is a multisig |
| **Guardian** | Veto queued proposals during timelock | Trusted security multisig; acts as last-resort safety |
| **Token holders** | Vote, propose (if above threshold) | Rational economic actors; hold tokens for governance |
| **Stellar network** | Ledger ordering, snapshot integrity | Stellar consensus is operating correctly |

### What the Admin CAN do
- Cancel any proposal (before or after voting)
- Deploy the initial contracts

### What the Admin CANNOT do
- Vote on behalf of others
- Execute proposals without timelock
- Change governance parameters (no `update_config()` exists)
- Mint or burn tokens (token admin is separate)

### What the Guardian CAN do
- Veto a queued proposal during the timelock window

### What the Guardian CANNOT do
- Create or vote on proposals
- Execute proposals
- Cancel proposals (that is the admin's role)

---

## Known Gaps

1. **No parameter upgrade path.** Once initialized, governance parameters (`quorum_bps`, `voting_period`, etc.) are permanent. There is no `update_config()`. A new contract instance must be deployed to change parameters, requiring token migration.

2. **No vote delegation.** Delegation is planned but not yet implemented. Currently, voting power is strictly non-transferable balance at the snapshot.

3. **No on-chain action execution.** The `execute` function marks the proposal as executed but does not dispatch arbitrary on-chain actions. This is a TODO in the contract.

4. **Single admin key.** The admin is a single address, not a multisig by default. Deployers should use a multisig or hardware wallet for the admin key.

5. **No vote change or withdrawal.** Once a vote is cast, it cannot be changed or withdrawn.

6. **Quorum is calculated at finalize time.** The quorum requirement is fixed at proposal creation based on total supply, but actual participation is checked at finalize. If total supply changes significantly between creation and finalize (e.g. large mint/burn), the quorum target may be stale.

---

## Recommendations for Production Deployment

1. Use a **multisig** for the admin key (e.g. 3-of-5).
2. Use a dedicated **security multisig** for the guardian role.
3. Deploy on **testnet first** and run through a full governance cycle before mainnet.
4. Monitor the `proposal_created`, `vote_cast`, and `proposal_finalized` events for anomalies.
5. Consider starting with a **higher quorum** (e.g. 10%) and lowering it as participation grows.
