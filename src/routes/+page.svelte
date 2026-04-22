<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { open } from '@tauri-apps/plugin-dialog';
	import { readFile, writeFile } from '$lib/tauri';
	import { workspace } from '$lib/workspace/store.svelte';
	import FileTree from '$lib/workspace/FileTree.svelte';
	import Editor from '$lib/editor/Editor.svelte';
	import ChatPanel from '$lib/ai/ChatPanel.svelte';
	import SettingsModal from '$lib/settings/SettingsModal.svelte';
	import { syncStore } from '$lib/sync/store.svelte';
	import { ragStore } from '$lib/rag/store.svelte';

	const WELCOME = `# Welcome to jsmde

Pick a workspace folder to get started, or open an individual \`.md\` file.

- **Cmd/Ctrl + S** — save
- **Cmd/Ctrl + O** — pick workspace folder
- Edits autosave 500 ms after you stop typing

---

_Next up: AI chat (M2) and Google Drive sync (M3)._
`;

	let docText = $state(WELCOME);
	let loadedPath = $state<string | null>(null);
	let dirty = $state(false);
	let status = $state('Ready');
	let saving = $state(false);
	let aiOpen = $state(true);
	let settingsOpen = $state(false);

	let saveTimer: ReturnType<typeof setTimeout> | null = null;

	onMount(() => {
		void workspace.init().then(async () => {
			if (workspace.openPath) {
				await openPath(workspace.openPath);
			}
		});
		void syncStore.init();
		void ragStore.init();
		return () => {
			workspace.teardown();
			syncStore.teardown();
		};
	});

	$effect(() => {
		// Reload the open file when the watcher fires a change and it's our file.
		// Only changeTick should drive this — loadedPath/docText/dirty changes
		// happen during normal open/new/edit and would race with in-flight loads.
		workspace.changeTick;
		untrack(() => {
			const path = loadedPath;
			if (!path) return;
			void (async () => {
				try {
					const next = await readFile(path);
					if (!dirty && next !== docText) {
						docText = next;
						status = `Reloaded · ${path}`;
					}
				} catch {
					/* file may have been deleted or moved */
				}
			})();
		});
	});

	async function pickWorkspace() {
		const picked = await open({ directory: true, multiple: false });
		if (!picked || typeof picked !== 'string') return;
		await workspace.setRoot(picked);
		status = `Workspace: ${picked}`;
	}

	async function openPath(path: string) {
		try {
			docText = await readFile(path);
			loadedPath = path;
			dirty = false;
			status = path;
			await workspace.setOpenPath(path);
		} catch (e) {
			status = `Open failed: ${e}`;
		}
	}

	async function pickFile() {
		const picked = await open({
			multiple: false,
			directory: false,
			filters: [{ name: 'Markdown', extensions: ['md', 'markdown', 'mdx', 'txt'] }]
		});
		if (!picked || typeof picked !== 'string') return;
		await openPath(picked);
	}

	function joinPath(parent: string, name: string): string {
		// If the path contains any backslash, treat it as Windows-native.
		const sep = parent.includes('\\') ? '\\' : '/';
		const trimmed = parent.replace(/[\\/]+$/, '');
		return `${trimmed}${sep}${name}`;
	}

	function uniqueUntitled(entries: { name: string }[]): string {
		const existing = new Set(entries.map((e) => e.name.toLowerCase()));
		let name = 'Untitled.md';
		if (!existing.has(name.toLowerCase())) return name;
		for (let i = 2; i < 10_000; i++) {
			name = `Untitled ${i}.md`;
			if (!existing.has(name.toLowerCase())) return name;
		}
		return `Untitled ${Date.now()}.md`;
	}

	let creating = $state(false);

	async function newFile() {
		if (creating) return;
		if (!workspace.root) {
			status = 'Open a workspace folder first to create a new file.';
			return;
		}
		creating = true;
		try {
			// Save any pending edits to the currently-open file first.
			if (loadedPath && dirty) {
				await save();
			}
			// Refresh so uniqueUntitled sees the latest tree state.
			await workspace.refresh();
			const name = uniqueUntitled(workspace.entries);
			const target = joinPath(workspace.root, name);
			await writeFile(target, '');
			docText = '';
			loadedPath = target;
			dirty = false;
			status = `New file · ${target}`;
			await workspace.setOpenPath(target);
			await workspace.refresh();
		} catch (e) {
			status = `New file failed: ${e}`;
		} finally {
			creating = false;
		}
	}

	function onEditorChange(next: string) {
		docText = next;
		dirty = true;
		if (saveTimer) clearTimeout(saveTimer);
		saveTimer = setTimeout(() => {
			void save();
		}, 500);
	}

	async function save() {
		if (!loadedPath) return;
		if (!dirty) return;
		saving = true;
		try {
			await writeFile(loadedPath, docText);
			dirty = false;
			const now = new Date();
			status = `Saved · ${now.toLocaleTimeString()} · ${loadedPath}`;
		} catch (e) {
			status = `Save failed: ${e}`;
		} finally {
			saving = false;
		}
	}

	async function saveAs() {
		const { save: saveDialog } = await import('@tauri-apps/plugin-dialog');
		const picked = await saveDialog({
			defaultPath: loadedPath ?? 'untitled.md',
			filters: [{ name: 'Markdown', extensions: ['md'] }]
		});
		if (!picked) return;
		try {
			await writeFile(picked, docText);
			loadedPath = picked;
			dirty = false;
			status = `Saved · ${picked}`;
			await workspace.setOpenPath(picked);
		} catch (e) {
			status = `Save failed: ${e}`;
		}
	}

	function onKeydown(e: KeyboardEvent) {
		const mod = e.metaKey || e.ctrlKey;
		if (!mod) return;
		if (e.key === 's') {
			e.preventDefault();
			if (loadedPath) void save();
			else void saveAs();
		} else if (e.key === 'o') {
			e.preventDefault();
			if (e.shiftKey) void pickFile();
			else void pickWorkspace();
		} else if (e.key === 'n') {
			e.preventDefault();
			void newFile();
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="flex h-screen flex-col">
	<header
		class="flex items-center justify-between border-b border-neutral-200 bg-white/70 px-4 py-2 backdrop-blur dark:border-neutral-800 dark:bg-neutral-950/70"
	>
		<div class="flex items-center gap-3">
			<span class="text-sm font-semibold tracking-tight">jsmde</span>
			<span class="text-xs text-neutral-500">
				{workspace.root ? workspace.root.split('/').pop() : 'no workspace'}
			</span>
			{#if saving}
				<span class="text-xs text-neutral-400">saving…</span>
			{:else if dirty}
				<span class="text-xs text-amber-600 dark:text-amber-400">modified</span>
			{/if}
		</div>
		<div class="flex items-center gap-2">
			<button
				class="rounded-md border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-700 dark:hover:bg-neutral-800"
				title={workspace.root
					? 'New file (⌘N)'
					: 'Open a workspace folder first'}
				onclick={newFile}
				disabled={!workspace.root || creating}
			>
				New
			</button>
			<button
				class="rounded-md border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800"
				onclick={pickWorkspace}
			>
				Open folder
			</button>
			<button
				class="rounded-md border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800"
				onclick={pickFile}
			>
				Open file
			</button>
			<button
				class="rounded-md bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-700 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
				onclick={() => (loadedPath ? save() : saveAs())}
			>
				Save
			</button>
			<button
				class="rounded-md border border-neutral-300 px-2 py-1 text-sm hover:bg-neutral-100 disabled:opacity-50 dark:border-neutral-700 dark:hover:bg-neutral-800"
				title="Sync with Google Drive"
				onclick={() => syncStore.runNow()}
				disabled={syncStore.running || !syncStore.gdrive.signed_in || !workspace.root}
			>
				{#if syncStore.running}
					<span class="animate-spin inline-block">↻</span> Syncing…
				{:else}
					↻ Sync
				{/if}
			</button>
			<button
				class="rounded-md border border-neutral-300 px-2 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800"
				title="Toggle AI panel"
				onclick={() => (aiOpen = !aiOpen)}
			>
				✨ AI
			</button>
			<button
				class="rounded-md border border-neutral-300 px-2 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800"
				title="Settings"
				onclick={() => (settingsOpen = true)}
			>
				⚙
			</button>
		</div>
	</header>

	<div class="grid min-h-0 flex-1" style="grid-template-columns: 240px 1fr {aiOpen ? '360px' : '0'};">
		<aside class="min-h-0 overflow-hidden border-r border-neutral-200 dark:border-neutral-800">
			<FileTree onSelect={openPath} />
		</aside>
		<main class="min-h-0 overflow-hidden">
			<Editor value={docText} onChange={onEditorChange} />
		</main>
		{#if aiOpen}
			<aside
				class="min-h-0 overflow-hidden border-l border-neutral-200 dark:border-neutral-800"
			>
				<ChatPanel docText={docText} onOpenSettings={() => (settingsOpen = true)} />
			</aside>
		{/if}
	</div>

	<footer
		class="flex items-center justify-between gap-3 border-t border-neutral-200 px-4 py-1 text-xs text-neutral-500 dark:border-neutral-800"
	>
		<span class="truncate">{status}</span>
		<span class="flex items-center gap-3">
			{#if syncStore.lastReport}
				<span
					class="truncate"
					title={`↑ ${syncStore.lastReport.uploaded} · ↓ ${syncStore.lastReport.downloaded} · conflicts ${syncStore.lastReport.conflicts}`}
				>
					last sync: ↑{syncStore.lastReport.uploaded} ↓{syncStore.lastReport.downloaded}
					{#if syncStore.lastReport.conflicts > 0}
						· <span class="text-amber-500">{syncStore.lastReport.conflicts} conflict{syncStore.lastReport.conflicts === 1 ? '' : 's'}</span>
					{/if}
				</span>
			{/if}
			{#if syncStore.errorText}
				<span class="text-red-500">{syncStore.errorText}</span>
			{/if}
			<span>{docText.length} chars</span>
		</span>
	</footer>
</div>

<SettingsModal open={settingsOpen} onClose={() => (settingsOpen = false)} />
