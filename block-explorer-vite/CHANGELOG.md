# Changelog

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
