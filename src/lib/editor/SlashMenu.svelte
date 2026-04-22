<script lang="ts">
	import type { SlashCommand } from './slashCommands';

	type Props = {
		open: boolean;
		rect: DOMRect | null;
		items: SlashCommand[];
		selectedIndex: number;
		onPick: (i: number) => void;
	};

	let { open, rect, items, selectedIndex, onPick }: Props = $props();

	const style = $derived.by(() => {
		if (!rect) return '';
		const top = rect.bottom + 6;
		const left = rect.left;
		return `top: ${top}px; left: ${left}px`;
	});

	$effect(() => {
		if (!open || selectedIndex < 0) return;
		const el = document.querySelector(
			`[data-slash-item="${selectedIndex}"]`
		) as HTMLElement | null;
		el?.scrollIntoView({ block: 'nearest' });
	});
</script>

{#if open && items.length > 0 && rect}
	<div
		class="slash-menu fixed z-50 max-h-72 w-64 overflow-auto rounded-md border border-neutral-300 bg-white shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
		style={style}
	>
		<ul class="py-1">
			{#each items as cmd, i (cmd.id)}
				<li>
					<button
						type="button"
						data-slash-item={i}
						class="flex w-full items-center gap-3 px-3 py-1.5 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 {i ===
						selectedIndex
							? 'bg-neutral-100 dark:bg-neutral-800'
							: ''}"
						onmousedown={(e) => {
							e.preventDefault();
							onPick(i);
						}}
					>
						<span
							class="flex h-6 w-6 shrink-0 items-center justify-center rounded border border-neutral-200 bg-neutral-50 text-[10px] font-semibold dark:border-neutral-700 dark:bg-neutral-800"
						>
							{cmd.icon}
						</span>
						<span class="flex min-w-0 flex-col">
							<span class="truncate text-neutral-900 dark:text-neutral-100">{cmd.title}</span>
							<span class="truncate text-[11px] text-neutral-500">{cmd.description}</span>
						</span>
					</button>
				</li>
			{/each}
		</ul>
	</div>
{/if}
