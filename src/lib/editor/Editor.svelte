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
		if (md === lastEmitted) return;
		lastEmitted = md;
		onChange(md);
	}

	$effect(() => {
		if (flushRef) flushRef(flushEmit);
	});

	let host: HTMLDivElement;
	let bubbleEl: HTMLDivElement | null = $state(null);
	let editor: Editor | null = $state(null);
	let lastEmitted = '';
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
		lastEmitted = toMarkdown(ed);
		suppressUpdate = false;
		editor = ed;
		return () => {
			ed.destroy();
			if (editor === ed) editor = null;
		};
	});

	$effect(() => {
		const nextValue = value;
		if (!editor) return;
		if (nextValue === lastEmitted) return;
		console.debug('[editor] setContent', nextValue.length, 'chars');
		suppressUpdate = true;
		fromMarkdown(editor, nextValue);
		lastEmitted = toMarkdown(editor);
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
