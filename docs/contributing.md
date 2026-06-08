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

Quorum participates in the Stellar Wave on Drips Network. Tagged issues earn
Points during each sprint cycle so contributors can identify which work is
rewarded before they start.

### Points Labels

Maintainers should add one points label to each approved issue:

| Label | Typical scope | Examples |
|---|---|---|
| `drips:1` | Small task | Docs edits, copy fixes, styling tweaks, good first issues |
| `drips:3` | Medium task | New component, focused test coverage, SDK method |
| `drips:5` | Large task | New page, contract function, integration flow |
| `drips:8` | Complex task | Full feature, security-sensitive work, multi-package change |

If an issue grows during implementation, discuss the new scope in the issue
before changing the label. Avoid adding more than one points label to the same
issue.

### Contributor Flow

1. Pick an open issue with a `drips:*` label and confirm it is not already
   assigned or claimed.
2. Comment `/attempt #ISSUE_NUMBER` with a short plan before starting work.
3. Keep the change focused on the issue acceptance criteria.
4. Run the relevant frontend, SDK, or contract checks and include the commands
   in the pull request.
5. Open a pull request with `/claim #ISSUE_NUMBER` and link the issue.

Pull requests without an issue link or claim marker may still be reviewed, but
they may not be tracked correctly by Drips Wave automation.

### Maintainer Setup Checklist

- Apply to the Stellar Wave program at <https://www.drips.network/wave>.
- Label approved issues with exactly one `drips:*` points label.
- Pin a "Drips Wave active" issue during active sprint cycles.
- Keep this guide updated if the reward program or sprint cadence changes.

## Code Style
- TypeScript: strict mode, no `any`
- Rust: follow Soroban SDK patterns, use `Result<T, Error>` over `panic!()`
- Commits: conventional commits format (`feat:`, `fix:`, `docs:`, `style:`, `test:`, `chore:`)
