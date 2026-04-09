# Contributing to SoroStream

> **SoroStream** is a real-time token streaming and vesting protocol built on Stellar Soroban.
> We welcome contributions of all kinds — from fixing a typo to implementing a non-linear vesting curve.

---

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Complexity Tiers & Rewards](#complexity-tiers--rewards)
3. [Prerequisites](#prerequisites)
4. [Getting Started](#getting-started)
5. [Repository Structure](#repository-structure)
6. [Branching & Commit Convention](#branching--commit-convention)
7. [Working on the Contract (`packages/contracts`)](#working-on-the-contract)
8. [Working on the Frontend (`packages/frontend`)](#working-on-the-frontend)
9. [Running All Tests](#running-all-tests)
10. [Pull Request Process](#pull-request-process)
11. [Security Disclosures](#security-disclosures)

---

## Code of Conduct

This project follows the [Contributor Covenant v2.1](https://www.contributor-covenant.org/version/2/1/code_of_conduct/).
By participating you agree to uphold a welcoming, harassment-free environment for everyone.

---

## Complexity Tiers & Rewards

Every open issue is tagged with a **complexity tier** that maps to a Drips Wave funding range.
Pick the tier that matches your experience level.

| Tier | Label | Examples | Reward Range |
|------|-------|----------|-------------|
| 🟢 Trivial | `complexity: trivial` | Doc comments, typos, config tweaks | $25 – $100 |
| 🟡 Medium | `complexity: medium` | React components, basic contract math | $200 – $600 |
| 🔴 High | `complexity: high` | Security logic, indexer, vesting curves | $800 – $2,500 |

> Rewards are distributed via the [Drips protocol](https://drips.network/) on Ethereum.

---

## Prerequisites

Install the following tools before cloning:

### 1. Rust & Soroban CLI

```bash
# Install rustup (Rust toolchain manager)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the WASM compilation target required by Soroban
rustup target add wasm32-unknown-unknown

# Install the Stellar CLI (includes `stellar contract build/deploy/invoke`)
cargo install --locked stellar-cli --version 21.0.0 --features opt

# Verify
stellar --version   # should print stellar 21.x.x
cargo --version     # should print 1.88+
```

### 2. Node.js & npm

```bash
# Install Node.js 20 LTS (use nvm to manage versions)
nvm install 20
nvm use 20

node --version  # v20.x.x
npm --version   # 10.x.x
```

### 3. Freighter Wallet (for frontend testing)

Install the [Freighter browser extension](https://freighter.app/) and switch it to **Testnet**.

---

## Getting Started

```bash
# 1. Fork the repository on GitHub, then clone your fork:
git clone https://github.com/YOUR_USERNAME/sorostream.git
cd sorostream

# 2. Install all npm workspace dependencies:
npm install

# 3. Copy the environment template and fill in your values:
cp .env.example .env.local
# Edit .env.local: set NEXT_PUBLIC_CONTRACT_ID after deploying the contract

# 4. Verify the Rust contract compiles:
cd packages/contracts
cargo build --target wasm32-unknown-unknown
cargo test

# 5. Return to root and start the frontend dev server:
cd ../..
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) to see the frontend.

---

## Repository Structure

```
sorostream/
├── .github/
│   ├── ISSUE_TEMPLATE/
│   │   ├── trivial_issue.yml       # Docs, typos, config — first-timer friendly
│   │   ├── medium_issue.yml        # Components, contract math
│   │   └── high_issue.yml          # Security, indexer, vesting curves
│   └── workflows/
│       └── ci.yml                  # CI: lint → test → WASM build
│
├── packages/
│   ├── contracts/                  # Rust / Soroban smart contract
│   │   ├── Cargo.toml
│   │   ├── rust-toolchain.toml     # Pinned Rust version
│   │   └── src/
│   │       └── lib.rs              # Core contract: initialize, create_stream, claim_stream
│   │
│   └── frontend/                   # Next.js 14 / TypeScript / Tailwind
│       ├── package.json
│       ├── next.config.ts
│       ├── tailwind.config.ts
│       └── src/
│           ├── app/                # App Router pages
│           ├── components/         # Shared React components
│           └── lib/                # Soroban client helpers
│
├── CONTRIBUTING.md                 # ← You are here
├── README.md
├── package.json                    # Turborepo root
└── turbo.json                      # Task graph definition
```

---

## Branching & Commit Convention

| Branch Pattern | Purpose |
|---|---|
| `main` | Stable, production-ready. Direct pushes prohibited. |
| `develop` | Integration branch. All PRs target this. |
| `feature/<name>` | New feature or enhancement. |
| `fix/<issue-number>-<short-desc>` | Bug fix linked to a GitHub Issue. |
| `docs/<name>` | Documentation-only changes. |
| `security/<name>` | Security fixes. Open a private advisory first. |

### Commit Message Format (Conventional Commits)

```
<type>(<scope>): <short summary>

[optional body]

[optional footer: Closes #<issue>]
```

**Types:** `feat`, `fix`, `docs`, `chore`, `refactor`, `test`, `ci`, `security`
**Scopes:** `contracts`, `frontend`, `ci`, `root`

**Examples:**
```
feat(contracts): implement cancel_stream with pro-rata refund
fix(frontend): correct vesting progress bar rounding error
docs(contributing): add indexer setup instructions
test(contracts): add test_cancel_at_50pct edge case
```

---

## Working on the Contract

> **File:** `packages/contracts/src/lib.rs`

### Environment Setup

```bash
cd packages/contracts

# Format code (run before every commit)
cargo fmt

# Lint (must be clean in CI)
cargo clippy --target wasm32-unknown-unknown -- -D warnings

# Run unit tests
cargo test

# Build the optimised WASM (what gets deployed on-chain)
stellar contract build --profile release
```

### Adding a New Contract Function (Step-by-Step)

1. **Data structures first.** If you need new state, add a `#[contracttype]` struct or enum
   variant *before* the `SoroStream` struct.  New `DataKey` variants go in the `DataKey` enum.

2. **Implement the function** inside `#[contractimpl] impl SoroStream { ... }`.
   Follow these patterns:

   ```rust
   pub fn my_new_fn(env: Env, caller: Address, stream_id: u64) {
       // 1. AUTH — require the caller's signature
       caller.require_auth();

       // 2. CHECKS — validate inputs and load state
       let mut stream: Stream = env.storage().persistent()
           .get(&DataKey::Stream(stream_id))
           .expect("stream not found");

       // 3. EFFECTS — update state BEFORE any token transfers
       stream.claimed_amount += some_value;
       env.storage().persistent().set(&DataKey::Stream(stream_id), &stream);

       // 4. INTERACTIONS — token transfers LAST (CEI pattern)
       let token_client = token::Client::new(&env, &stream.token);
       token_client.transfer(&env.current_contract_address(), &caller, &some_value);

       // 5. EVENT — so off-chain indexers can react
       env.events().publish(
           (symbol_short!("stream"), symbol_short!("my_evt")),
           (stream_id, caller, some_value),
       );
   }
   ```

3. **Write tests** for every new function — minimum:
   - One **happy path** test
   - One test for each `panic!` / error condition
   - One test for the boundary/edge case (e.g. zero amount, already cancelled)

4. **Run the full suite:**
   ```bash
   cargo fmt && cargo clippy --target wasm32-unknown-unknown -- -D warnings && cargo test
   ```

### Contract Invariants (Never Break These)

| Invariant | Description |
|-----------|-------------|
| **CEI Order** | State changes always happen *before* token transfers |
| **Auth First** | `address.require_auth()` is called at the *top* of every mutating function |
| **No Double Claim** | `claimed_amount` only increases, never decreases |
| **Cancel is Final** | Once `is_cancelled = true`, no further state changes to that stream |
| **Token Balance** | Sum of all `(total_amount - claimed_amount)` for active streams ≤ contract token balance |

---

## Working on the Frontend

> **Directory:** `packages/frontend/`

### Environment Setup

```bash
cd packages/frontend

# Install (already done from root `npm install`)

# Start development server
npm run dev

# Lint
npm run lint

# Type check
npx tsc --noEmit

# Production build
npm run build
```

### Adding a New Page

1. Create `packages/frontend/src/app/<route>/page.tsx`
2. Export a `default` function component and a `metadata` object:
   ```tsx
   import type { Metadata } from 'next';
   export const metadata: Metadata = { title: 'My Page — SoroStream' };
   export default function MyPage() { ... }
   ```
3. Gate wallet-required content with the `useFreighter` hook (see `lib/hooks/useFreighter.ts`).

### Adding a New Component

1. Create `packages/frontend/src/components/<ComponentName>.tsx`
2. Add `"use client"` at the top **only** if the component uses React hooks or browser APIs.
3. Read-only Soroban queries use `simulateTransaction` (no gas, no wallet signature required).
   See `lib/sorobanClient.ts` for helpers.

### Accessing the Contract from the Frontend

```ts
// lib/sorobanClient.ts pattern:
import { Contract, SorobanRpc, TransactionBuilder, Networks, BASE_FEE } from '@stellar/stellar-sdk';

// Read-only call — uses simulateTransaction, no signature needed
export async function getClaimableAmount(streamId: bigint): Promise<bigint> {
  const server = new SorobanRpc.Server(process.env.NEXT_PUBLIC_RPC_URL!);
  const contract = new Contract(process.env.NEXT_PUBLIC_CONTRACT_ID!);
  // ... build and simulate tx
}

// State-changing call — user signs with Freighter
export async function claimStream(streamId: bigint): Promise<string> {
  // ... build tx, sign with Freighter, submit
}
```

---

## Running All Tests

```bash
# --- From the repo root ---

# Run everything via Turborepo (lint + build + contract tests)
npm run build
npm run lint

# --- Contract tests only ---
cd packages/contracts
cargo test
# With verbose output:
cargo test -- --nocapture

# --- Frontend type check only ---
cd packages/frontend
npx tsc --noEmit

# --- CI equivalent (exactly what GitHub Actions runs) ---
npm ci
npx turbo run lint
npx turbo run build
cd packages/contracts && cargo fmt --check && cargo clippy --target wasm32-unknown-unknown -- -D warnings && cargo test
```

---

## Pull Request Process

1. **Open a GitHub Issue first** using the appropriate template (trivial / medium / high).
   Discuss the approach before writing code — especially for High issues.

2. **Branch** off `develop`:
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b feature/my-feature
   ```

3. **Implement**, following the guidelines above.

4. **Ensure the PR checklist passes:**

   **For contract changes:**
   - [ ] `cargo fmt` — no formatting changes uncommitted
   - [ ] `cargo clippy --target wasm32-unknown-unknown -- -D warnings` — zero warnings
   - [ ] `cargo test` — all tests green
   - [ ] New tests added for every new function and every panic path
   - [ ] Doc comments on all new public functions (`///`)

   **For frontend changes:**
   - [ ] `npm run lint` — zero ESLint errors
   - [ ] `npx tsc --noEmit` — zero type errors
   - [ ] `npm run build` — production build succeeds

   **For all PRs:**
   - [ ] CI is green on your branch
   - [ ] PR description links to the GitHub Issue (`Closes #NNN`)
   - [ ] No unrelated changes in the diff

5. **Open the PR** against `develop` (not `main`).
   Fill in the PR template — especially the *Security Impact* section for contract changes.

6. **Request review** from a core maintainer.
   High-complexity PRs require sign-off from **two** maintainers.

7. Maintainers squash-merge to `develop` to keep a linear history.

---

## Security Disclosures

> ⚠️ **Do NOT open public GitHub Issues for security vulnerabilities.**

Use GitHub's [Private Security Advisory](https://github.com/YOUR_ORG/sorostream/security/advisories/new)
feature to report vulnerabilities confidentially.  We will coordinate a fix and credit you in
the release notes before public disclosure.

For general security questions, email **security@sorostream.xyz** (replace with your actual contact).
