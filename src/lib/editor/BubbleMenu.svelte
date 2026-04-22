<script lang="ts">
	import type { Editor } from '@tiptap/core';

	type Props = {
		editor: Editor | null;
		bindEl: (el: HTMLDivElement) => void;
	};

	let { editor, bindEl }: Props = $props();

	let tick = $state(0);
	let menuEl: HTMLDivElement;

	$effect(() => {
		if (menuEl) bindEl(menuEl);
	});

	$effect(() => {
		if (!editor) return;
		const onUpdate = () => (tick = tick + 1);
		editor.on('selectionUpdate', onUpdate);
		editor.on('transaction', onUpdate);
		return () => {
			editor.off('selectionUpdate', onUpdate);
			editor.off('transaction', onUpdate);
		};
	});

	const isActive = (name: string, attrs?: Record<string, unknown>): boolean => {
		void tick;
		if (!editor) return false;
		return attrs ? editor.isActive(name, attrs) : editor.isActive(name);
	};

	function toggleLink() {
		if (!editor) return;
		const prev = editor.getAttributes('link').href as string | undefined;
		if (prev) {
			editor.chain().focus().extendMarkRange('link').unsetLink().run();
			return;
		}
		const url = window.prompt('URL', 'https://');
		if (!url) return;
		editor.chain().focus().extendMarkRange('link').setLink({ href: url }).run();
	}

	type BtnProps = {
		on: () => void;
		active: boolean;
		title: string;
		children: import('svelte').Snippet;
	};
</script>

{#snippet btn(p: BtnProps)}
	<button
		type="button"
		class="flex h-7 min-w-[28px] items-center justify-center rounded px-1.5 text-xs transition hover:bg-neutral-200 dark:hover:bg-neutral-800 {p.active
			? 'bg-neutral-200 text-neutral-900 dark:bg-neutral-700 dark:text-white'
			: 'text-neutral-700 dark:text-neutral-200'}"
		title={p.title}
		onmousedown={(e) => e.preventDefault()}
		onclick={p.on}
	>
		{@render p.children()}
	</button>
{/snippet}

<div
	bind:this={menuEl}
	class="bubble-menu z-50 flex items-center gap-0.5 rounded-md border border-neutral-300 bg-white p-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-900"
	style="display: none"
>
	{#if editor}
		{@render btn({
			on: () => editor?.chain().focus().toggleBold().run(),
			active: isActive('bold'),
			title: 'Bold (⌘B)',
			children: boldIcon
		})}
		{@render btn({
			on: () => editor?.chain().focus().toggleItalic().run(),
			active: isActive('italic'),
			title: 'Italic (⌘I)',
			children: italicIcon
		})}
		{@render btn({
			on: () => editor?.chain().focus().toggleStrike().run(),
			active: isActive('strike'),
			title: 'Strikethrough',
			children: strikeIcon
		})}
		{@render btn({
			on: () => editor?.chain().focus().toggleCode().run(),
			active: isActive('code'),
			title: 'Inline code',
			children: codeIcon
		})}
		<div class="mx-1 h-4 w-px bg-neutral-300 dark:bg-neutral-700"></div>
		{@render btn({
			on: () => editor?.chain().focus().toggleHeading({ level: 1 }).run(),
			active: isActive('heading', { level: 1 }),
			title: 'Heading 1',
			children: h1Icon
		})}
		{@render btn({
			on: () => editor?.chain().focus().toggleHeading({ level: 2 }).run(),
			active: isActive('heading', { level: 2 }),
			title: 'Heading 2',
			children: h2Icon
		})}
		{@render btn({
			on: () => editor?.chain().focus().toggleHeading({ level: 3 }).run(),
			active: isActive('heading', { level: 3 }),
			title: 'Heading 3',
			children: h3Icon
		})}
		<div class="mx-1 h-4 w-px bg-neutral-300 dark:bg-neutral-700"></div>
		{@render btn({
			on: toggleLink,
			active: isActive('link'),
			title: 'Link',
			children: linkIcon
		})}
	{/if}
</div>

{#snippet boldIcon()}<span class="font-bold">B</span>{/snippet}
{#snippet italicIcon()}<span class="italic">I</span>{/snippet}
{#snippet strikeIcon()}<span class="line-through">S</span>{/snippet}
{#snippet codeIcon()}<span class="font-mono">{'</>'}</span>{/snippet}
{#snippet h1Icon()}<span class="font-semibold">H1</span>{/snippet}
{#snippet h2Icon()}<span class="font-semibold">H2</span>{/snippet}
{#snippet h3Icon()}<span class="font-semibold">H3</span>{/snippet}
{#snippet linkIcon()}<span>🔗</span>{/snippet}
