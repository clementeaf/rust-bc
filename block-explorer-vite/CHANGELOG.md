# Changelog

## 2026-04-23

### EVM execution (revm integration)

- Full EVM via `revm` v38 — deploy, call, and static-call Solidity contracts
- Endpoints: `POST /evm/deploy`, `POST /evm/call`, `POST /evm/static-call`, `GET /evm/contracts`
- 14 unit tests: rapid deploys, gas exhaustion, oversized bytecode, infinite loops
- 10 stress + fuzz tests: concurrent threads, proptest randomized bytecode/calldata/addresses
- Rust nightly upgraded to 2026-04-20 (rustc 1.97)

### Landing page

- New full-width landing at `/` — hero, 4 capability pillars, tech specs, dual CTA
- Dashboard moved to `/dashboard`
- Landing renders outside Layout (no sidebar)

### Full Spanish localization

- Translated all remaining English strings: table headers, buttons, status badges, empty states, error messages across 12 pages
- Replaced local `shortAddr` helpers with shared `shortHash` from `lib/format.ts`

## 2026-04-22

### Rebrand — Cerulean Ledger

- Renamed from "rust-bc" to "Cerulean Ledger" across all UI surfaces
- Unified DID prefix to `did:cerulean:` (was `did:bc:` and `did:rustbc:`)
- Footer updated: "DLT post-cuantica · Soberania digital"

### Refactor — shared utilities and lazy routes

- Extracted `lib/format.ts`: `timeAgo`, `shortHash`, `shortCode`, `fmtDate`, `formatExpiry`
- Extracted `lib/routes.ts`: route config array with `React.lazy` + `Suspense`
- Simplified `main.tsx` from 15 manual imports to a single loop

### Cleanup — presentation-ready

- Full Spanish localization (placeholders, buttons, error messages)
- Removed Governance page (incomplete, "coming soon")
- Deleted orphan `Governance.tsx`
