# Kale Predict
Prediction market for *Kale* contract invocations

[Project design on Canva](https://www.canva.com/design/DAGrEeM1_q8/fFBLvGngyR9Uj-OEKXtj3A/edit)

![Kale Predict Smart Contract](https://github.com/user-attachments/assets/40486c68-5ecf-4a5e-bc55-9a1c65e3ac25)

## Why
- **Demonstrate a minimal prediction-market pattern on Soroban.**  
  Kale Predict escrows stakes, settles against on-chain data, and splits a pool—using only the standard SDK and a single custom contract.
- **Keep the moving parts small.**  
  One admin sets and resolves rounds; users call just five functions — `start_round`, `bet`, `resolve_round`, `claim`, `refund`.
- **Serve as a reference implementation.**  
  The repo links a Rust contract, generated TypeScript bindings, and a React 19 front-end that can be adapted for any over/under metric.

## How it works (TL;DR)

| Actor | Can do | On-chain calls |
|-------|--------|----------------|
| **Admin** (single account) | • **Start** a round (`predicted_count`, `deadline_ledger`, `finality_ledger`)<br>• **Resolve** after finality, posting the actual count & winning side | `start_round()`<br>`resolve_round()` |
| **Betting users** (anyone) | **Before** `deadline_ledger`<br>• Bet KALE on **Higher** or **Lower**<br><br>**After** resolution<br>• **Claim** winnings (winners split the losing pool pro-rata)<br><br>**If admin ghosts**<br>• **Refund** stake after `finality_ledger + 100` ledgers | `bet()`<br>`claim()`<br>`refund()` |

Token transfers occur **only** on `bet`, `claim`, and `refund`; admin calls are state-only.
