<script lang="ts">
	import { workspace } from './store.svelte';
	import { listDir, type DirEntry } from '$lib/tauri';

	type Props = {
		onSelect: (path: string) => void;
	};

	let { onSelect }: Props = $props();

	let expanded = $state<Record<string, boolean>>({});
	let children = $state<Record<string, DirEntry[]>>({});

	// The workspace store's watcher listener handles refresh on changeTick;
	// no need to duplicate it here.

	function isMarkdown(e: DirEntry) {
		if (e.is_dir) return true;
		const n = e.name.toLowerCase();
		return n.endsWith('.md') || n.endsWith('.markdown') || n.endsWith('.mdx');
	}

	async function toggle(entry: DirEntry) {
		if (!entry.is_dir) {
			onSelect(entry.path);
			return;
		}
		const next = !expanded[entry.path];
		expanded[entry.path] = next;
		if (next && !children[entry.path]) {
			try {
				children[entry.path] = await listDir(entry.path);
			} catch {
				children[entry.path] = [];
			}
		}
	}

	function visible(entries: DirEntry[]): DirEntry[] {
		return entries.filter(
			(e) => isMarkdown(e) && !e.name.startsWith('.')
		);
	}
</script>

{#snippet node(entry: DirEntry, depth: number)}
	{@const isOpen = expanded[entry.path]}
	{@const selected = workspace.openPath === entry.path}
	<button
		class="flex w-full items-center gap-1.5 truncate px-2 py-1 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 {selected
			? 'bg-neutral-200 dark:bg-neutral-800'
			: ''}"
		style="padding-left: {depth * 12 + 8}px"
		onclick={() => toggle(entry)}
		title={entry.path}
	>
		{#if entry.is_dir}
			<span class="inline-block w-3 text-neutral-400">{isOpen ? '▾' : '▸'}</span>
			<span class="truncate">{entry.name}</span>
		{:else}
			<span class="inline-block w-3"></span>
			<span class="truncate">{entry.name}</span>
		{/if}
	</button>
	{#if entry.is_dir && isOpen && children[entry.path]}
		{#each visible(children[entry.path]) as child (child.path)}
			{@render node(child, depth + 1)}
		{/each}
	{/if}
{/snippet}

<nav class="h-full overflow-auto py-2 text-sm">
	{#if !workspace.root}
		<p class="px-3 text-neutral-500">No workspace open.</p>
	{:else}
		<div class="px-3 pb-2 text-[11px] uppercase tracking-wider text-neutral-500">
			{workspace.root.split('/').pop() || workspace.root}
		</div>
		{#each visible(workspace.entries) as entry (entry.path)}
			{@render node(entry, 0)}
		{/each}
	{/if}
</nav>
