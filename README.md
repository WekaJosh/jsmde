# jsmde

Simple, beautiful, lightweight markdown editor. Tauri 2 + SvelteKit + Tailwind 4.

**Status:** M4 — MVP complete. Open/edit/autosave locally, chat with Claude or any AI provider, sync to Google Drive.

## Features

- **WYSIWYG editor** — TipTap 3 with markdown round-trip
- **Workspace folder** — pick any directory, file tree with live watch via `notify`
- **Autosave** — 500 ms debounce, safe external-change reload
- **AI chat sidebar** — Anthropic, OpenAI, Google, Ollama. Keys in OS keychain. Streaming via Tauri events. Doc-context toggle.
- **Google Drive sync** — OAuth PKCE (bring your own client ID), conflict-aware reconciliation with `.conflict-<ISO>.md` sidecars, SQLite metadata DB
- **Lightweight** — Tauri 2 shell, release profile tuned for small bundles

## Install

Download the appropriate installer from [Releases](https://github.com/WekaJosh/jsmde/releases).

### macOS

The DMGs are unsigned, so Gatekeeper will say the app is "damaged and can't be opened". It isn't — macOS just refuses to run unsigned downloads. After dragging the app into `/Applications`, run this once in Terminal:

```sh
xattr -rd com.apple.quarantine /Applications/jsmde.app
```

Then launch normally.

- `jsmde_*_aarch64.dmg` — Apple Silicon (M1/M2/M3/M4)
- `jsmde_*_x64.dmg` — Intel

### Windows

Run `jsmde_*_x64-setup.exe` or `jsmde_*_x64_en-US.msi`. The binaries are unsigned; on the SmartScreen warning click "More info" → "Run anyway".

### Linux

Pick `.AppImage`, `.deb`, or `.rpm` depending on your distribution.

## Keyboard

- `Cmd/Ctrl + S` save
- `Cmd/Ctrl + O` open workspace folder
- `Cmd/Ctrl + Shift + O` open individual file

## Roadmap

- **M0 ✅** scaffold
- **M1 ✅** WYSIWYG editor, workspace, file tree, autosave, watcher
- **M2 ✅** AI chat sidebar with multi-provider support
- **M3 ✅** Google Drive bidirectional sync with conflict handling
- **M4 ✅** Updater plugin wired, release profile, RELEASE.md playbook
- **M5** iOS/Android (Tauri 2 mobile), Notion/SharePoint backends, CRDT collab, embeddings/RAG
