# jsmde

Simple, beautiful, lightweight markdown editor. Tauri 2 + SvelteKit + Tailwind 4.

**Status:** M4 — MVP complete. Open/edit/autosave locally, chat with Claude or any AI provider, sync to Google Drive.

Full plan: `~/.claude/plans/i-need-a-markdown-keen-dove.md`.
Release steps: [RELEASE.md](RELEASE.md).

## Run

```sh
pnpm install
pnpm tauri dev        # desktop app with hot reload
```

Browser-only shell (no Tauri APIs wired):

```sh
pnpm dev
```

## Build

```sh
pnpm tauri build      # .dmg / .msi / .AppImage, <20MB target
```

## Quality gates

```sh
pnpm check            # svelte-check + tsc
pnpm test             # vitest
cd src-tauri && cargo check
```

## Features

- **WYSIWYG editor** — TipTap 3 with markdown round-trip
- **Workspace folder** — pick any directory, file tree with live watch via `notify`
- **Autosave** — 500 ms debounce, safe external-change reload
- **AI chat sidebar** — Anthropic, OpenAI, Google, Ollama. Keys in OS keychain. Streaming via Tauri events. Doc-context toggle.
- **Google Drive sync** — OAuth PKCE (bring your own client ID), conflict-aware reconciliation with `.conflict-<ISO>.md` sidecars, SQLite metadata DB
- **Lightweight** — Tauri 2 shell, release profile tuned for small bundles

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
