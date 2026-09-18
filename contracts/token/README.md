# QUORUM Token Contract

SEP-41 compatible governance token for the Quorum protocol.

## Functions
- `initialize` — set name, symbol, decimals, initial supply
- `transfer` — transfer tokens between accounts
- `mint` — admin-only token minting
- `burn` — holder-authorized token burning
- `approve` — set an allowance for a spender, valid until `expiration_ledger`
- `transfer_from` — spender moves tokens on behalf of an owner, against an allowance
- `allowance` — remaining allowance, or 0 once the approval has lapsed
- `get_past_balance` — balance as of a past ledger, backing snapshot voting power

## Allowance expiry

`approve(owner, spender, amount, expiration_ledger)` stores the expiry
alongside the amount. An allowance is spendable up to and including
`expiration_ledger` and reads as 0 from the ledger after.

Per SEP-41, a live approval (`amount > 0`) is rejected with
`InvalidExpiration` if it would expire in the past — which makes
`expiration_ledger: 0` invalid for any real ledger. A zero `amount` is a
revocation and is accepted with any expiry, so an owner can always cancel an
approval.

Spending part of an allowance keeps the original expiry: a partial spend must
not extend the life of the remainder.
