# ⚖️ Settlement Contract — Soroban / Stellar

> **Holds legal settlement payments in escrow and releases funds automatically once all parties have signed the agreement.**

---

## 📋 Project Description

`settlement-contract` is a **Soroban smart contract** deployed on the Stellar blockchain that acts as a trustless escrow for legal settlement payments. Traditionally, releasing settlement funds requires costly intermediaries (escrow agents, law firm trust accounts, clearing banks) and significant delays. This contract eliminates that friction by encoding the release condition directly on-chain: **funds are transferred to the payee one and only one time — when both disputing parties and a designated arbiter have cryptographically signed the agreement**.

The contract is well-suited for:
- Out-of-court civil settlements
- Insurance claim payouts
- Employment dispute resolutions
- Commercial contract disputes
- Any scenario where a "signed-before-paid" guarantee is required

---

## 🔍 What It Does

```
Payor deposits funds                 Both parties sign           Arbiter confirms → Funds released
──────────────────────►  ESCROW  ──────────────────────────►  ARBITER CHECK  ──────────────────►  PAYEE
        (locked)                       (on-chain record)             (final gate)
```

### Lifecycle

| Step | Who | Action |
|------|-----|--------|
| 1 | **Payor** | Calls `initialize()` — deposits the settlement amount into the contract's escrow account along with the case reference, payee, and arbiter addresses |
| 2 | **Payor** | Calls `sign()` — records their cryptographic signature on the agreement |
| 3 | **Payee** | Calls `sign()` — records their cryptographic signature on the agreement |
| 4 | **Arbiter** | Calls `arbiter_release()` — verifies both parties have signed and triggers the token transfer to the payee |

> **Cancel path:** Before any signatures are collected, the payor may call `cancel()` to reclaim the deposited funds.

---

## ✨ Features

### 🔐 Dual-Party Signature Gate
Funds are frozen in the contract until **both** the payor and payee have independently signed. No single party can unilaterally release the money — not even the party who deposited it.

### 🧑‍⚖️ Neutral Arbiter Control
A designated arbiter (mediator, law firm, court officer) holds the **final release key**. Even if both parties sign, the arbiter must call `arbiter_release()` to execute the transfer, providing a human oversight layer.

### 🏦 Non-Custodial Escrow
Funds live in the smart contract address itself — not in any third-party wallet. The payor, payee, and arbiter never hold the funds simultaneously. The contract is the escrow agent.

### 💱 Any Stellar Asset
The `token` parameter accepts any **Stellar Asset Contract (SAC)** address — USDC, XLM, or any custom token — making the contract currency-agnostic.

### 🔄 Safe Cancellation
If negotiations break down before signatures are collected, the payor can call `cancel()` to reclaim funds. Once even one party has signed, cancellation is blocked, preventing bad-faith withdrawal after agreement.

### 📡 On-Chain Event Emission
Every major state transition emits a Soroban event:
- `settlement_created` — escrow funded
- `party_signed` — a party recorded their signature
- `settlement_released` — funds transferred to payee
- `settlement_cancelled` — escrow refunded to payor

These events allow off-chain legal systems, dashboards, and notification services to react in real time.

### 🔒 One-Time Initialisation
The contract can only be initialised once. Re-deploying a fresh contract instance per case keeps each settlement fully isolated.

### 🧪 Tested
Ships with a unit test suite covering:
- Full happy-path: deposit → sign × 2 → arbiter release → funds received
- Guard: arbiter cannot release without both signatures
- Cancel: payor reclaims funds when no signatures have been collected

---

## 🗂️ Project Structure

```
settlement-contract/
├── Cargo.toml          # Soroban SDK dependency & release profile
└── src/
    └── lib.rs          # Contract implementation + tests
```

---

## 🚀 Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- Soroban CLI: `cargo install --locked soroban-cli`
- A funded Stellar testnet account

### Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled WASM artifact will be at:
```
target/wasm32-unknown-unknown/release/settlement_contract.wasm
```

### Run Tests

```bash
cargo test
```

### Deploy to Testnet

```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/settlement_contract.wasm \
  --source <YOUR_SECRET_KEY> \
  --network testnet
```

### Invoke — Initialise Settlement

```bash
soroban contract invoke \
  --source <PAYOR_SECRET_KEY> \
  --network testnet \
  -- initialize \
  --payor  <PAYOR_ADDRESS> \
  --payee  <PAYEE_ADDRESS> \
  --arbiter <ARBITER_ADDRESS> \
  --token  <TOKEN_CONTRACT_ADDRESS> \
  --amount 50000000 \
  --case_id "CASE-2024-001"
```

---

## 📜 Contract Interface

| Function | Caller | Description |
|----------|--------|-------------|
| `initialize(payor, payee, arbiter, token, amount, case_id)` | Payor | Fund escrow and create settlement record |
| `sign(signer)` | Payor or Payee | Record on-chain signature |
| `arbiter_release(arbiter)` | Arbiter | Release funds after both parties sign |
| `cancel()` | Payor | Reclaim funds (only before any signatures) |
| `get_settlement()` | Anyone | Return full settlement struct |
| `get_status()` | Anyone | Return current `SettlementStatus` |
| `is_fully_signed()` | Anyone | Return `true` if both parties have signed |

---

## ⚠️ Disclaimer

This contract is provided for educational and prototyping purposes. It has **not** been audited. Do not use in production with real funds without a professional security review.

---

Wallet Address: GBNL55KHIZU2XM6QHHIWFMITMJ7QEJD63UT3EMTMEHSP7NW7CA276QKJ

Contract Address: CAWUXGHGWMGNUV2GPBDZOF6722EN4V67O36ACYFA7Z3WJ3FSOKUJPKLQ

https://stellar.expert/explorer/testnet/contract/CAWUXGHGWMGNUV2GPBDZOF6722EN4V67O36ACYFA7Z3WJ3FSOKUJPKLQ

<img width="1679" height="770" alt="image" src="https://github.com/user-attachments/assets/88023091-09d4-4f24-bad0-0244cc22f8f1" />


