# Releasing jsmde

## One-time setup

### 1. Generate an updater signing keypair

Private key stays on the release machine / in CI secrets. Public key goes in `tauri.conf.json`.

```sh
pnpm tauri signer generate -w ~/.tauri/jsmde.key
```

This creates:
- `~/.tauri/jsmde.key` — the private key (keep secret)
- `~/.tauri/jsmde.key.pub` — the public key

Copy the contents of `jsmde.key.pub` into `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.

Set the password as `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` and the key content as `TAURI_SIGNING_PRIVATE_KEY` during `tauri build`.

### 2. macOS code signing + notarization

- Developer ID Application certificate in your keychain
- Export env vars for `tauri build`:
  - `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD` (for CI), or
  - `APPLE_SIGNING_IDENTITY` locally
  - `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` for notarization

### 3. Windows code signing

- EV cert recommended; configure `APPLE_CERTIFICATE`-style env vars for Authenticode. See Tauri docs for the current env var names.

### 4. Google OAuth client ID for Drive sync

Each user provides their own. In Google Cloud Console:

1. Create a project, enable the Google Drive API.
2. Create an OAuth consent screen (External), add yourself as a test user.
3. Create an OAuth 2.0 Client ID of type "Desktop app".
4. The client ID is what users paste into Settings → Cloud. PKCE is used, so no client secret is needed.

## Building

Dev:

```sh
pnpm tauri dev
```

Production bundle for the host platform:

```sh
pnpm tauri build
```

Output goes to `src-tauri/target/release/bundle/`:
- macOS: `.dmg` and `.app`
- Windows: `.msi` and `.exe`
- Linux: `.AppImage` and `.deb`

## Distributing

Upload release artifacts to GitHub Releases. Add a `latest.json` manifest alongside, following the [Tauri updater schema](https://v2.tauri.app/plugin/updater/#server-support), and set `plugins.updater.endpoints` in `tauri.conf.json` to point at it.

## CI

A GitHub Actions workflow using `tauri-apps/tauri-action` can build all three platforms in parallel and publish artifacts. This repo does not include one yet — add `.github/workflows/release.yml` when ready.
