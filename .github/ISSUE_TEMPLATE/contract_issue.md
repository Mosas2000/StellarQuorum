---
name: Smart contract issue
about: Report a bug, enhancement, or behavior issue in Soroban smart contracts
title: ""
labels: ["contracts"]
assignees: ""
---

### Contract

- [ ] Governance Contract (`contracts/governance`)
- [ ] Token Contract (`contracts/token`)
- [ ] Other

### Affected Function(s)

<!-- Please specify the exact contract function(s) affected (e.g., initialize, propose, vote, finalize, execute, cancel, transfer, approve, transfer_from, get_vote). -->

### Description

<!-- A clear and concise description of the contract issue or requested modification. -->

### Expected Behavior

<!-- What the contract should do when the function is invoked. -->

### Actual Behavior

<!-- What actually happened (e.g., unexpected trap/panic, incorrect state or event, authorization failure, storage TTL expiration). -->

### Steps to Reproduce / Reproduction Test Case

<!-- Provide reproduction steps, Soroban SDK test code, or Stellar CLI command: -->

```rust
// Soroban test reproducing the behavior
```

### Environment and Network

- **Network**: [Local Standalone | Testnet | Futurenet | Mainnet]
- **Soroban SDK Version**: 
- **Stellar CLI Version**: 
- **Rust Toolchain**: 

### Severity

- [ ] Critical (vulnerability, locked tokens, state corruption)
- [ ] High (auth failure, incorrect voting tally, blocked proposal execution)
- [ ] Medium (incorrect parameter handling, missing event emission)
- [ ] Low / Refactor (gas optimization, code clarity, documentation)

### Additional Context

<!-- Simulation XDR, contract IDs, transaction hashes, or related proposal IDs. -->
