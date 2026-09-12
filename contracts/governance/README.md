# Quorum Governance Contract

Soroban smart contract implementing on-chain governance for the Quorum protocol.

## Functions
- `initialize` — set config, quorum params, token address
- `create_proposal` — create a new proposal (requires token balance >= threshold)
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
