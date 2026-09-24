# Choosing Governance Parameters

Quorum's governance parameters are set once at initialization and cannot be changed. This guide explains the trade-offs for each parameter and provides recommended starting values.

## Parameters

### `quorum_bps` — Quorum Threshold

**What it is:** The minimum total voting power (For + Against + Abstain) required for a proposal to be eligible for execution, expressed in basis points of circulating token supply.

**Trade-offs:**
- **Too low (e.g. 100 = 1%):** A small group can pass proposals that affect everyone. Governance is easy to capture.
- **Too high (e.g. 5000 = 50%):** Governance becomes nearly impossible. Proposals fail because not enough holders participate.
- **Sweet spot:** Enough to ensure broad participation, low enough to allow legitimate proposals to pass.

**Recommended starting value:** `500` (5%)

**Reasoning:** 5% is a common starting point for token-weighted governance. It's high enough to prevent small-group capture but low enough that active proposals can reach quorum. Adjust based on observed participation rates — if most proposals fail due to quorum, lower it; if proposals pass with suspiciously few voters, raise it.

**How to revisit:** After 6 months, analyze the ratio of proposals that reached quorum vs. those that failed. If >50% fail due to quorum, consider lowering to 300 (3%). If <10% fail, you could raise to 750 (7.5%).

---

### `voting_period` — Voting Window

**What it is:** How long voting remains open after a proposal is created, measured in ledgers. At Stellar's ~5 second close time, 17,280 ledgers ≈ 1 day.

**Trade-offs:**
- **Too short (e.g. 3,600 = ~5 hours):** Token holders in other timezones or with busy schedules miss the vote. Decentralization suffers.
- **Too long (e.g. 86,400 = ~5 days):** Decision-making is slow. Market conditions may change during the vote, making the proposal stale.
- **Sweet spot:** Long enough for global participation, short enough to maintain momentum.

**Recommended starting value:** `17,280` (≈ 24 hours)

**Reasoning:** 24 hours covers all major timezones at least once. For higher-stakes proposals (parameter changes, treasury movements), consider 51,840 (~3 days) by deploying a second governance instance with a longer window.

**How to revisit:** Check vote distribution across hours. If votes cluster in one timezone, increase the period.

---

### `timelock_period` — Timelock Delay

**What it is:** The mandatory waiting period between a proposal passing and its execution, measured in ledgers. At ~5 seconds per ledger, 8,640 ledgers ≈ 12 hours.

**Trade-offs:**
- **Too short (e.g. 1,800 = ~2.5 hours):** The guardian has insufficient time to review and veto. Security is compromised.
- **Too long (e.g. 34,560 = ~2 days):** Execution is delayed. Market conditions may change, making the action suboptimal.
- **Sweet spot:** Enough time for the guardian to review, short enough that execution is timely.

**Recommended starting value:** `8,640` (≈ 12 hours)

**Reasoning:** 12 hours gives the guardian multisig a full business day to review a queued proposal and decide whether to veto. This is a reasonable balance between security and execution speed.

**How to revisit:** If the guardian consistently vetoes within the first hour, the timelock could be shortened. If the guardian needs more review time, lengthen it.

---

### `proposal_threshold` — Minimum Proposal Stake

**What it is:** The minimum token balance a proposer must hold to create a proposal. Measured in the token's smallest unit (7 decimals). Example: `500,000,000,000` = 50,000 QUORUM.

**Trade-offs:**
- **Too low (e.g. 1,000,000 = 0.1 QUORUM):** Spam proposals flood the queue. Holders must wade through noise.
- **Too high (e.g. 10,000,000,000,000 = 1,000,000 QUORUM):** Only the largest holders can propose. Governance becomes oligarchic.
- **Sweet spot:** High enough to deter spam, low enough that mid-size holders can participate.

**Recommended starting value:** `500,000,000,000` (50,000 QUORUM)

**Reasoning:** 50,000 QUORUM (0.05% of 100M supply) is meaningful enough to signal commitment but accessible to community members, not just whales.

**How to revisit:** If spam is a problem, raise the threshold. If legitimate proposals are being blocked because proposers can't meet it, lower it.

---

## Summary Table

| Parameter | Recommended | ≈ Real-World | Trade-off |
|-----------|-------------|--------------|-----------|
| `quorum_bps` | 500 | 5% of supply | Participation vs. capture resistance |
| `voting_period` | 17,280 | ~24 hours | Global access vs. speed |
| `timelock_period` | 8,640 | ~12 hours | Security review vs. execution speed |
| `proposal_threshold` | 500,000,000,000 | 50,000 QUORUM | Spam prevention vs. accessibility |

## Governance Instance Strategy

Since parameters are immutable, consider deploying **multiple governance instances** for different proposal types:

- **Standard governance:** Default parameters (24h vote, 12h timelock)
- **Emergency governance:** Shorter voting period (6h) + shorter timelock (2h) for urgent fixes
- **Treasury governance:** Longer voting period (3d) + higher threshold for large fund movements

Each instance is independent — token holders vote separately on each.
