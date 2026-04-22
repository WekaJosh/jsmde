<script lang="ts">
	import { Editor } from '@tiptap/core';
	import { onDestroy, untrack } from 'svelte';
	import { buildExtensions, type MenuElements } from './extensions';
	import { fromMarkdown, toMarkdown } from './markdown';
	import BubbleMenu from './BubbleMenu.svelte';
	import SlashMenu from './SlashMenu.svelte';
	import type { SlashCommand, SlashHandler } from './slashCommands';

	type Props = {
		value: string;
		onChange: (next: string) => void;
		flushRef?: (flush: () => void) => void;
	};

	let { value, onChange, flushRef }: Props = $props();

	const EMIT_DEBOUNCE_MS = 400;
	let emitTimer: ReturnType<typeof setTimeout> | null = null;

	function flushEmit() {
		if (emitTimer) {
			clearTimeout(emitTimer);
			emitTimer = null;
		}
		if (!editor) return;
		const md = toMarkdown(editor);
		if (md === lastSynced) return;
		// Update the shared baseline BEFORE propagating to the parent.
		// When the parent's reactive `value` change re-fires the content-sync
		// effect, nextValue === lastSynced will be true and the effect skips.
		// Breaks the feedback loop that otherwise forms when tiptap-markdown's
		// round-trip isn't stable on large documents.
		lastSynced = md;
		onChange(md);
	}

	$effect(() => {
		if (flushRef) flushRef(flushEmit);
	});

	let host: HTMLDivElement;
	let bubbleEl: HTMLDivElement | null = $state(null);
	let editor: Editor | null = $state(null);
	// lastSynced is the content string that the editor is currently showing
	// (as would be produced by toMarkdown). Both the content-sync effect and
	// flushEmit write to it; the content-sync effect skips when its input
	// matches, breaking setContent/onUpdate feedback loops.
	let lastSynced = '';
	let suppressUpdate = false;

	let slashOpen = $state(false);
	let slashRect = $state<DOMRect | null>(null);
	let slashItems = $state<SlashCommand[]>([]);
	let slashSelected = $state(0);
	let slashPick: (i: number) => void = () => {};

	const slashHandler: SlashHandler = {
		onOpen: ({ rect, items, selectedIndex }) => {
			slashItems = items;
			slashRect = rect;
			slashSelected = selectedIndex;
			slashOpen = true;
		},
		onUpdate: ({ rect, items, selectedIndex }) => {
			slashItems = items;
			slashRect = rect;
			slashSelected = selectedIndex;
		},
		onClose: () => {
			slashOpen = false;
			slashItems = [];
			slashRect = null;
		},
		bindPick: (fn) => {
			slashPick = fn;
		}
	};

	function pickSlashItem(i: number) {
		slashPick(i);
	}

	function ensureBubbleVisible(el: HTMLDivElement) {
		bubbleEl = el;
	}

	$effect(() => {
		if (!host) return;
		if (!bubbleEl) return;
		const initial = untrack(() => value);
		const menus: MenuElements = {
			bubble: bubbleEl,
			slash: slashHandler
		};
		const ed = new Editor({
			element: host,
			extensions: buildExtensions(menus),
			content: '',
			editorProps: {
				attributes: {
					class:
						'prose prose-neutral dark:prose-invert max-w-none focus:outline-none min-h-full'
				}
			},
			onUpdate: () => {
				if (suppressUpdate) return;
				// Debounce serialization — toMarkdown is O(doc size), so running
				// it per keystroke melts the CPU on large files. Let edits settle
				// briefly, then flush. Cmd+S flushes synchronously via flushRef.
				if (emitTimer) clearTimeout(emitTimer);
				emitTimer = setTimeout(flushEmit, EMIT_DEBOUNCE_MS);
			}
		});
		suppressUpdate = true;
		ed.commands.setContent(initial, { emitUpdate: false });
		lastSynced = initial;
		suppressUpdate = false;
		editor = ed;

		const teardownVisibility = setupBlockVisibility(ed);

		return () => {
			teardownVisibility();
			ed.destroy();
			if (editor === ed) editor = null;
		};
	});

	// Explicit render buffer for large documents. Blocks far from the viewport
	// get `content-visibility: auto`, so the browser skips their layout. Blocks
	// within 2000px of the viewport get .in-range, which forces normal layout
	// so fast scroll after a resize doesn't flash placeholder boxes.
	function setupBlockVisibility(ed: Editor): () => void {
		const dom = ed.view.dom as HTMLElement;
		const observed = new Set<Element>();
		const io = new IntersectionObserver(
			(entries) => {
				for (const e of entries) {
					e.target.classList.toggle('in-range', e.isIntersecting);
				}
			},
			{ rootMargin: '2000px 0px 2000px 0px' }
		);
		const syncObservations = () => {
			const current = new Set<Element>(Array.from(dom.children));
			for (const el of observed) {
				if (!current.has(el)) {
					io.unobserve(el);
					observed.delete(el);
				}
			}
			for (const el of current) {
				if (!observed.has(el)) {
					io.observe(el);
					observed.add(el);
				}
			}
		};
		syncObservations();
		const mo = new MutationObserver(syncObservations);
		mo.observe(dom, { childList: true });
		return () => {
			io.disconnect();
			mo.disconnect();
			observed.clear();
		};
	}

	$effect(() => {
		const nextValue = value;
		if (!editor) return;
		if (nextValue === lastSynced) return;
		suppressUpdate = true;
		fromMarkdown(editor, nextValue);
		// Track the INPUT we just loaded, not the round-tripped serialization.
		// If the round-trip isn't lossless, subsequent flushEmit will update
		// lastSynced to the actual serialized form, and the pair converges.
		lastSynced = nextValue;
		suppressUpdate = false;
	});

	onDestroy(() => {
		if (emitTimer) clearTimeout(emitTimer);
		editor?.destroy();
		editor = null;
	});
</script>

<div class="flex h-full flex-col overflow-auto">
	<div bind:this={host} class="mx-auto w-full max-w-3xl flex-1 px-8 py-10"></div>
</div>

<BubbleMenu {editor} bindEl={ensureBubbleVisible} />

<SlashMenu
	open={slashOpen}
	rect={slashRect}
	items={slashItems}
	selectedIndex={slashSelected}
	onPick={pickSlashItem}
/>

<style>
	:global(.ProseMirror) {
		min-height: 100%;
	}
	/* Render buffer for large docs: blocks far from the viewport get their
	   layout/paint skipped. An IntersectionObserver adds .in-range to blocks
	   within 2000px of the viewport so scroll never reveals an unlaid block. */
	:global(.ProseMirror > *:not(.in-range)) {
		content-visibility: auto;
		contain-intrinsic-size: auto 40px;
	}
	:global(.ProseMirror p.is-editor-empty:first-child::before) {
		color: #a3a3a3;
		content: attr(data-placeholder);
		float: left;
		height: 0;
		pointer-events: none;
	}
	:global(.ProseMirror .tableWrapper) {
		overflow-x: auto;
		margin: 1em 0;
	}
	:global(.ProseMirror table) {
		border-collapse: collapse;
		table-layout: fixed;
		width: 100%;
		margin: 0;
		overflow: hidden;
	}
	:global(.ProseMirror table td),
	:global(.ProseMirror table th) {
		border: 1px solid #d4d4d4;
		padding: 0.5em 0.75em;
		vertical-align: top;
		box-sizing: border-box;
		position: relative;
		min-width: 1em;
	}
	:global(.ProseMirror table th) {
		background: #f5f5f5;
		font-weight: 600;
		text-align: left;
	}
	@media (prefers-color-scheme: dark) {
		:global(.ProseMirror table td),
		:global(.ProseMirror table th) {
			border-color: #404040;
		}
		:global(.ProseMirror table th) {
			background: #262626;
			color: #f5f5f5;
		}
	}
	:global(.ProseMirror table .selectedCell) {
		background: rgba(59, 130, 246, 0.15);
	}
	:global(.ProseMirror table .column-resize-handle) {
		position: absolute;
		right: -2px;
		top: 0;
		bottom: 0;
		width: 4px;
		background: #60a5fa;
		pointer-events: none;
	}
	:global(.ProseMirror.resize-cursor) {
		cursor: col-resize;
	}
</style>
