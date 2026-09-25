# Quorum Governance Contract

Soroban smart contract implementing on-chain governance for the Quorum protocol.

## Functions
- `initialize` — set config, quorum params, token address
- `create_proposal` — create a new proposal (requires live token balance >= `proposal_threshold`)
- `vote` — cast For/Against/Abstain vote on active proposal
- `finalize` — evaluate quorum after voting period ends
- `execute` — execute queued proposal after timelock
- `cancel` — cancel by proposer or admin

## Quorum derivation

`quorum_required` is computed once, at proposal creation, by cross-invoking
`total_supply()` on the token contract recorded in `Config::token`:

```
quorum_required = total_supply * quorum_bps / 10000
```

`quorum_bps` is in basis points, so 500 = 5%. Integer division truncates, so
the threshold is never rounded above what the supply supports. The value is
frozen into the proposal, meaning later mints or burns cannot move the bar for
a proposal that is already open.

## Which balance is read, and when

Two different reads, deliberately:

| Check | Balance read | Why |
|---|---|---|
| `proposal_threshold` in `create_proposal` | **live**, at creation | Gates who may open a proposal *now*, so spamming the queue costs real stake. |
| Voting power in `vote` | **snapshot**, at `snapshot_ledger` | Fixed when the proposal opened, so tokens bought or borrowed mid-vote carry no weight. |

The snapshot read is backed by per-address balance checkpoints in the token
contract (`get_past_balance`). A voter with no power at the snapshot is
rejected with `NoVotingPower` rather than recording a zero-weight vote.
