# Quickstart: Nostr Identity Profile

## Purpose

Plan and implement the first Nostr profile slice: a Play Game `Profile` group with `Set Name (Username)` and a Nostr-authorized username prompt, without adding a custom game profile database.

## Implementation Checklist

1. Add Nostr DTOs and validation rules.
2. Add non-sensitive profile snapshot persistence and sensitivity tests.
3. Add a Nostr profile service wrapper using `nostr-sdk` or a mock/local adapter behind the same contract.
4. Add Play Game `Profile` group and `Set Name ()` button.
5. Add username prompt with exact `username:`, `Submit`, and `Cancel` copy.
6. Wire Submit to Nostr QR authorization and profile metadata publish.
7. Keep Cancel and failure paths profile-preserving.
8. Verify Lightning auth/payment prompts remain separate.

## Focused Commands

```powershell
cargo test -p lightning-service nostr
cargo test -p ui nostr
cargo check -p ui --target wasm32-unknown-unknown
cargo check -p web --target wasm32-unknown-unknown
cargo check -p desktop
```

## Browser Verification

```powershell
.\Scripts\Common\RunWeb.ps1
```

Verify:

- Play Game shows a `Profile` group.
- Button starts as `Set Name ()`.
- Clicking opens a prompt with `username:`, `Submit`, and `Cancel`.
- Invalid usernames are rejected without publishing.
- Cancel leaves the previous name unchanged.
- Successful mock/real Nostr auth updates the button to `Set Name (alice)`.
- Buy/Sell item still uses Lightning auth and does not open the Nostr profile prompt.

## Research Notes

- Nostr profile metadata is the durable profile source; local app storage is only a non-sensitive snapshot.
- `nostr-sdk` is the planned crate because it is the maintained high-level Rust Nostr SDK and documents metadata publishing plus wasm32 support.
- Current implementation uses the shared Nostr profile service contract with a mock/local QR authorization adapter for browser gameplay and a native-targeted `nostr-sdk` boundary. The direct SDK dependency is not compiled for `wasm32-unknown-unknown` because its current dependency stack requires wasm random/crypto configuration beyond this app's plain wasm check.
