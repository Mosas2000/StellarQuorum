# Contributing to Quorum

Thank you for your interest in contributing to Quorum!

## Ways to Contribute

### Add a Contract Feature
Open issues tagged `contract` are the best starting point. To add a feature:
1. Modify `contracts/governance/src/lib.rs` or `contracts/token/src/lib.rs`
2. Write tests using soroban-sdk testutils
3. Update the contract README

### Improve the Frontend
Issues tagged `frontend` cover UI improvements, new pages, and accessibility work:
1. Work in `frontend/app/` and `frontend/components/`
2. Run `npm run dev` to preview changes
3. Run `npm run build` before opening a PR to verify no build errors

### Extend the SDK
Issues tagged `sdk` cover new methods and improved typing:
1. Work in `sdk/src/`
2. Wire methods to real Soroban RPC calls
3. Export new types from `sdk/src/index.ts`

## Drips Wave

Quorum participates in the Stellar Wave on Drips Network. Tagged issues earn Points redeemable for XLM rewards during each monthly sprint cycle.

### Point Labels

Maintainers use `drips:*` labels to show the expected reward size before work starts:

| Label | Typical scope |
|---|---|
| `drips:1` | Small documentation, style, copy, or good-first-issue fixes |
| `drips:3` | Medium UI components, focused SDK work, or test coverage |
| `drips:5` | Larger pages, contract functions, or multi-file integrations |
| `drips:8` | Complex features, security-sensitive contract work, or full workflows |

If an issue is missing a point label, ask for sizing before opening a PR. During an active sprint, maintainers should pin a Drips Wave tracking issue that lists the eligible tasks for that cycle.

## Code Style
- TypeScript: strict mode, no `any`
- Rust: follow Soroban SDK patterns, use `Result<T, Error>` over `panic!()`
- Commits: conventional commits format (`feat:`, `fix:`, `docs:`, `style:`, `test:`, `chore:`)
