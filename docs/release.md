# Release process

Tagged release workflow (plan §12 Phase 8): pushing a `v*` tag builds
desktop installers for macOS (both architectures), Windows, and Linux, and
publishes them as GitHub Release assets alongside a `SHA256SUMS.txt`.

## Cutting a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers `.github/workflows/release.yml`. It can also be run manually
via **Actions → Release → Run workflow**, passing an already-pushed tag —
useful for exercising the pipeline without cutting a new tag.

## What the workflow produces

Four parallel `build` jobs, each producing installers via
[`tauri-apps/tauri-action`](https://github.com/tauri-apps/tauri-action):

| Job | Platform | Installer(s) |
|---|---|---|
| `macos-aarch64` | macOS, Apple Silicon | `.dmg`, `.app` |
| `macos-x86_64` | macOS, Intel | `.dmg`, `.app` |
| `windows` | Windows | NSIS `.exe` |
| `linux` | Linux (Ubuntu 22.04 build) | `.deb`, AppImage |

Every job uploads its installers to a **draft** GitHub Release for the tag
(draft, so a still-building platform never makes a partial release look
final). A `finalize-release` job then runs once all four builds succeed: it
downloads every uploaded asset, computes `SHA256SUMS.txt`, uploads that
file, and un-drafts (publishes) the release.

**Done when** (plan §12): a `v*` tag produces attached installers, and a
downloaded installer's hash matches the published one.

## Verifying a downloaded installer

Download the installer for your platform and `SHA256SUMS.txt` from the same
release, into the same directory, then:

```bash
sha256sum -c SHA256SUMS.txt --ignore-missing
```

`--ignore-missing` skips the lines for installers you didn't download; the
one you did download must print `OK`.

## Installers are unsigned

No Apple notarization, no Windows Authenticode signing
(`docs/decisions/0009-unsigned-release-installers.md`). First launch will
show an OS warning:

- **macOS**: "…cannot be opened because the developer cannot be verified."
  Right-click the app → **Open** → **Open** again in the confirmation
  dialog. (Or: System Settings → Privacy & Security → "Open Anyway".)
- **Windows**: SmartScreen "Windows protected your PC." Click **More info**
  → **Run anyway**.

Neither warning indicates a corrupted download — verify with
`SHA256SUMS.txt` if in doubt, not by dismissing the warning alone.

## Required secrets

None beyond the default `GITHUB_TOKEN` (used for creating the release and
uploading assets). No signing certificates are configured — see the
decision record above if that changes in a future phase.
