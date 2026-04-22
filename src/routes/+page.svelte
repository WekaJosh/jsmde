<script lang="ts">
	import { onMount } from 'svelte';
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

	let loading = $state(false);
	let loadingFile = $state<string | null>(null);
	let loadingStartMs = $state(0);
	let loadingElapsedMs = $state(0);

	let saveTimer: ReturnType<typeof setTimeout> | null = null;
	let loadingTimer: ReturnType<typeof setInterval> | null = null;

	function startLoading(fileName: string | null) {
		loading = true;
		loadingFile = fileName;
		loadingStartMs = Date.now();
		loadingElapsedMs = 0;
		if (loadingTimer) clearInterval(loadingTimer);
		loadingTimer = setInterval(() => {
			loadingElapsedMs = Date.now() - loadingStartMs;
		}, 100);
	}

	function stopLoading() {
		loading = false;
		loadingFile = null;
		loadingElapsedMs = 0;
		if (loadingTimer) {
			clearInterval(loadingTimer);
			loadingTimer = null;
		}
	}

	function basename(path: string): string {
		const parts = path.split(/[\\/]/);
		return parts[parts.length - 1] || path;
	}

	function fmtElapsed(ms: number): string {
		if (ms < 1000) return `${ms}ms`;
		return `${(ms / 1000).toFixed(1)}s`;
	}

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
		// Reload the open file when the watcher fires a change and it's our file
		workspace.changeTick;
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

	async function pickWorkspace() {
		const picked = await open({ directory: true, multiple: false });
		if (!picked || typeof picked !== 'string') return;
		await workspace.setRoot(picked);
		status = `Workspace: ${picked}`;
	}

	async function openPath(path: string) {
		startLoading(basename(path));
		try {
			const content = await readFile(path);
			docText = content;
			loadedPath = path;
			dirty = false;
			status = path;
			await workspace.setOpenPath(path);
			// Wait one animation frame so the editor's synchronous setContent
			// (triggered by the docText change) has a chance to complete before
			// we hide the loading indicator.
			await new Promise<void>((r) => requestAnimationFrame(() => r()));
		} catch (e) {
			status = `Open failed: ${e}`;
		} finally {
			stopLoading();
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
		startLoading(null);
		try {
			if (loadedPath && dirty) {
				await save();
			}
			await workspace.refresh();
			const name = uniqueUntitled(workspace.entries);
			const target = joinPath(workspace.root, name);
			loadingFile = name;
			await writeFile(target, '');
			docText = '';
			loadedPath = target;
			dirty = false;
			status = `New file · ${target}`;
			await workspace.setOpenPath(target);
			await workspace.refresh();
			await new Promise<void>((r) => requestAnimationFrame(() => r()));
		} catch (e) {
			status = `New file failed: ${e}`;
		} finally {
			creating = false;
			stopLoading();
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

	let flushEditor: (() => void) | null = null;

	async function save() {
		// Pull in any pending edits from the debounced serializer.
		flushEditor?.();
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
		flushEditor?.();
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
	<div
		class="pointer-events-none fixed top-0 left-0 right-0 z-50 h-[2px] overflow-hidden"
		aria-hidden="true"
	>
		{#if loading}
			<div class="progress-bar h-full bg-blue-500 dark:bg-blue-400"></div>
		{/if}
	</div>
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
				class="rounded-md border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800"
				title="Save (⌘S)"
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
				class="rounded-md border px-2 py-1 text-sm {aiOpen
					? 'border-neutral-400 bg-neutral-100 dark:border-neutral-600 dark:bg-neutral-800'
					: 'border-neutral-300 hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800'}"
				title={aiOpen ? 'Close AI panel' : 'Open AI panel'}
				aria-pressed={aiOpen}
				onclick={() => (aiOpen = !aiOpen)}
			>
				✨ AI
			</button>
			<button
				class="rounded-md border border-neutral-300 px-2 py-1 text-base leading-none hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800"
				title="Settings"
				aria-label="Settings"
				onclick={() => (settingsOpen = true)}
			>
				⚙️
			</button>
		</div>
	</header>

	<div class="grid min-h-0 flex-1" style="grid-template-columns: 240px 1fr {aiOpen ? '360px' : '0'};">
		<aside class="min-h-0 overflow-hidden border-r border-neutral-200 dark:border-neutral-800">
			<FileTree onSelect={openPath} />
		</aside>
		<main class="min-h-0 overflow-hidden">
			<Editor
				value={docText}
				onChange={onEditorChange}
				flushRef={(fn) => (flushEditor = fn)}
			/>
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
		<span class="flex min-w-0 items-center gap-2 truncate">
			{#if loading}
				<span class="inline-block animate-spin text-blue-600 dark:text-blue-400">⟳</span>
				<span class="truncate">
					Loading{loadingFile ? ` ${loadingFile}` : '…'}
				</span>
				<span class="shrink-0 tabular-nums text-neutral-400">{fmtElapsed(loadingElapsedMs)}</span>
			{:else}
				<span class="truncate">{status}</span>
			{/if}
		</span>
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

<style>
	.progress-bar {
		width: 40%;
		animation: progress-slide 1.1s ease-in-out infinite;
		will-change: transform, margin-left;
	}
	@keyframes progress-slide {
		0% {
			margin-left: -40%;
			width: 40%;
		}
		50% {
			margin-left: 30%;
			width: 50%;
		}
		100% {
			margin-left: 100%;
			width: 40%;
		}
	}
</style>
