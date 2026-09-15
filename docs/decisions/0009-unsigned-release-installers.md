# 0009 — Unsigned release installers for Phase 8

**Date:** 2026-09-15

## Decision

The tagged release workflow (`.github/workflows/release.yml`) produces
**unsigned** installers: no macOS notarization (Apple Developer ID + an
app-specific password), no Windows Authenticode signing. Plan §12 Phase 8
requires only that a `v*` tag produce attached installers with published
SHA-256 hashes — it does not require signing.

## Alternatives

- **Sign and notarize now.** Requires an Apple Developer Program
  membership and a Windows code-signing certificate, both paid, plus
  storing the credentials as GitHub Actions secrets. Owner confirmed
  neither is available yet.
- **Block Phase 8 on acquiring certificates.** Rejected — the plan's "done
  when" criterion doesn't require signing, and the SHA-256-published
  installer already gives an operator a way to verify what they downloaded
  matches what the tag produced, independent of OS code-signing trust.

## Reason

Owner decision, asked directly during Phase 8 planning: build unsigned now,
add signing later as its own change once certificates exist. Practical
consequence, stated up front so it isn't a surprise at release time:
unsigned installers trigger Gatekeeper ("unidentified developer") on macOS
and SmartScreen ("Windows protected your PC") on Windows on first launch.
Neither blocks installation; both require an explicit user override
(right-click → Open on macOS, "More info" → "Run anyway" on Windows).
`docs/release.md` documents this for whoever cuts the first release.
