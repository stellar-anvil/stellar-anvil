# Stellar Anvil — Stellar Wave Program Submission

## Project Summary

Stellar Anvil is a Foundry/Hardhat-equivalent local testing and simulation framework
for Soroban smart contracts on the Stellar blockchain. It addresses the single biggest
gap in the Stellar developer experience: the absence of a fast, batteries-included test
toolkit comparable to what EVM developers take for granted.

## Problem Statement

Every team building on Stellar/Soroban currently:

- Manually manipulates ledger structs to test time-sensitive logic
- Has no CPU/memory profiler to catch gas regressions before deployment
- Writes ad-hoc wiring for multi-contract interaction tests
- Has no mock wallet (Freighter) for auth testing beyond `env.mock_all_auths()`
- Has no way to test SEP-10 challenge/response flows in isolation
- Has no tool to diff ledger snapshots between deployments

This results in under-tested contracts, surprise gas costs on mainnet, and
months of duplicated effort across Wave-enrolled teams.

## Solution

Five composable Rust crates + a CLI binary:

| Crate | What it provides |
|---|---|
| `anvil-core` | `SandboxEnv`: named accounts, time travel, checkpoints, budget tracking |
| `anvil-profiler` | `Profiler`, `ProfileReport`, `BudgetThreshold`, `ProfileComparison` |
| `anvil-multi` | `ContractRegistry`, `Scenario`, `CallTrace`, `CallSequence` |
| `anvil-auth` | `MockWallet`, `MockSep10Server`, `MockInvoker`, `AuthPolicy` |
| `anvil-cli` | `stellar-anvil test/snapshot/profile/compare/info` |

## Impact on the Wave Ecosystem

Every repo currently enrolled in the Stellar Wave Program can immediately benefit:

- **DeFi protocols** (flowfi, BridgeWise, soroban-playground) — use `Scenario` for
  round-trip deposit/withdraw tests and `BudgetThreshold` to prevent fee regressions
- **NFT/registry contracts** (Soroban-Registry, Sanctifier) — use `ContractRegistry`
  dependency trees and `ProfileComparison` to track complexity growth
- **Auth-heavy contracts** — use `MockWallet` and `AuthPolicy` to cover rejection paths
  and ensure correct signers are required
- **Any contract with time-based logic** — use `sandbox.advance_time()` and
  `warp_to_sequence()` instead of fragile manual ledger setup

## Crate Metrics

- 5 library crates + 1 binary crate
- 3 fully-tested example contracts (counter, token+vault, time-lock vault)
- 40+ unit/integration tests across the examples
- Zero network calls required — fully offline testing

## Repository

https://github.com/your-org/stellar-anvil

## Requested Complexity / Points

**High** — this is developer infrastructure, not a dApp. It requires:
- Deep knowledge of the soroban-sdk testutils internals
- Cross-contract interaction modeling
- Rust panic catching and budget API wrapping
- CLI design and output formatting
- Full example contract suite

## Contact

Open an issue on GitHub or reach out via the Stellar Developer Discord.
