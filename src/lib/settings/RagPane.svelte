<script lang="ts">
	import { ragStore } from '$lib/rag/store.svelte';
	import { ragClear, ragReindex } from '$lib/rag/client';
	import { workspace } from '$lib/workspace/store.svelte';

	let busy = $state(false);
	let status = $state<string | null>(null);

	async function toggle(value: boolean) {
		busy = true;
		status = null;
		try {
			await ragStore.setEnabled(value);
			await ragStore.refresh(workspace.root);
			status = value
				? 'RAG enabled. Click Reindex to build the index.'
				: 'RAG disabled. Indexed data is retained — Clear removes it.';
		} catch (e) {
			status = `Failed: ${e}`;
		} finally {
			busy = false;
		}
	}

	async function reindex() {
		if (!workspace.root) {
			status = 'Open a workspace folder first.';
			return;
		}
		busy = true;
		status = 'Indexing…';
		try {
			const report = await ragReindex(workspace.root);
			await ragStore.refresh(workspace.root);
			const err = report.error ? ` (${report.error})` : '';
			status = `Indexed ${report.files_indexed} file(s), ${report.chunks_written} chunk(s)${err}.`;
		} catch (e) {
			status = `Indexing failed: ${e}`;
		} finally {
			busy = false;
		}
	}

	async function clear() {
		if (!workspace.root) return;
		busy = true;
		try {
			await ragClear(workspace.root);
			await ragStore.refresh(workspace.root);
			status = 'Cleared index for this workspace.';
		} catch (e) {
			status = `Clear failed: ${e}`;
		} finally {
			busy = false;
		}
	}

	function fmtTime(ms: number | null): string {
		if (!ms) return 'never';
		const d = new Date(ms);
		return d.toLocaleString();
	}
</script>

<div class="space-y-4 p-4">
	<div>
		<div class="flex items-center justify-between">
			<div>
				<div class="text-sm font-medium">Vault RAG</div>
				<div class="text-[11px] text-neutral-500">
					Embeds your markdown via Ollama so chat can pull relevant notes as context. Entirely local.
				</div>
			</div>
			<label class="inline-flex items-center gap-2">
				<input
					type="checkbox"
					checked={ragStore.enabled}
					disabled={busy}
					onchange={(e) => toggle((e.currentTarget as HTMLInputElement).checked)}
				/>
				<span class="text-sm">{ragStore.enabled ? 'On' : 'Off'}</span>
			</label>
		</div>
	</div>

	<div class="rounded border border-neutral-200 p-3 text-xs dark:border-neutral-800">
		<div class="flex justify-between">
			<span class="text-neutral-500">Model</span>
			<span>{ragStore.model || '—'}</span>
		</div>
		<div class="mt-1 flex justify-between">
			<span class="text-neutral-500">Ollama endpoint</span>
			<span class="truncate pl-2">{ragStore.ollamaUrl || '—'}</span>
		</div>
		<div class="mt-1 flex justify-between">
			<span class="text-neutral-500">Files indexed</span>
			<span>{ragStore.filesIndexed}</span>
		</div>
		<div class="mt-1 flex justify-between">
			<span class="text-neutral-500">Chunks</span>
			<span>{ragStore.chunks}</span>
		</div>
		<div class="mt-1 flex justify-between">
			<span class="text-neutral-500">Last indexed</span>
			<span>{fmtTime(ragStore.lastIndexedMs)}</span>
		</div>
	</div>

	{#if ragStore.progress && ragStore.indexing}
		<div class="text-[11px] text-neutral-500">
			{ragStore.progress.phase}: {ragStore.progress.done}/{ragStore.progress.total}
			{#if ragStore.progress.current}— {ragStore.progress.current}{/if}
		</div>
	{/if}

	<div class="flex gap-2">
		<button
			class="rounded-md bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-700 disabled:opacity-50 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
			disabled={busy || !ragStore.enabled || !workspace.root}
			onclick={reindex}
		>
			Reindex now
		</button>
		<button
			class="rounded-md border border-red-300 px-3 py-1 text-sm text-red-700 hover:bg-red-50 disabled:opacity-50 dark:border-red-800 dark:text-red-300 dark:hover:bg-red-900/30"
			disabled={busy || !workspace.root || ragStore.chunks === 0}
			onclick={clear}
		>
			Clear index
		</button>
	</div>

	{#if !workspace.root}
		<div class="text-[11px] text-amber-600 dark:text-amber-400">
			Open a workspace folder to enable indexing.
		</div>
	{/if}

	{#if status}
		<div class="text-[11px] text-neutral-500">{status}</div>
	{/if}

	<div class="text-[11px] text-neutral-500">
		Requires Ollama running locally with
		<code class="rounded bg-neutral-100 px-1 dark:bg-neutral-800">{ragStore.model || 'nomic-embed-text'}</code>
		pulled. Run <code class="rounded bg-neutral-100 px-1 dark:bg-neutral-800">ollama pull {ragStore.model || 'nomic-embed-text'}</code> in a terminal if you haven't.
	</div>
</div>
