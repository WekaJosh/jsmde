<script lang="ts">
	import { aiStore } from './store.svelte';
	import { PROVIDERS } from './providers';
	import { chatStream, type ChatMessage, type ChatStreamHandle } from './client';
	import { ragStore } from '$lib/rag/store.svelte';
	import { ragSearch } from '$lib/rag/client';
	import { workspace } from '$lib/workspace/store.svelte';
	import { renderMarkdown } from '$lib/editor/markdown';

	type Props = {
		docText: string;
		onOpenSettings: () => void;
	};

	let { docText, onOpenSettings }: Props = $props();

	type UIMessage = ChatMessage & { streaming?: boolean };

	let messages = $state<UIMessage[]>([]);
	let input = $state('');
	let busy = $state(false);
	let errorText = $state<string | null>(null);
	let handle: ChatStreamHandle | null = null;
	let scrollHost: HTMLDivElement | undefined;

	const info = $derived(PROVIDERS[aiStore.provider]);

	$effect(() => {
		messages.length;
		if (scrollHost) scrollHost.scrollTop = scrollHost.scrollHeight;
	});

	function clamp(text: string, max: number) {
		if (text.length <= max) return text;
		return '[…truncated…]\n' + text.slice(text.length - max);
	}

	async function send() {
		const trimmed = input.trim();
		if (!trimmed || busy) return;
		errorText = null;
		input = '';
		const userMsg: UIMessage = { role: 'user', content: trimmed };
		const assistantMsg: UIMessage = { role: 'assistant', content: '', streaming: true };
		messages = [...messages, userMsg, assistantMsg];
		busy = true;

		let vaultBlock = '';
		if (ragStore.enabled && ragStore.useVaultContext && workspace.root) {
			try {
				const hits = await ragSearch(workspace.root, trimmed, 5);
				if (hits.length > 0) {
					const chunks = hits
						.map(
							(h) =>
								`<note path="${h.rel_path}" score="${h.score.toFixed(3)}">\n${h.content}\n</note>`
						)
						.join('\n\n');
					vaultBlock = `Relevant notes from the user's vault follow. Cite by path when useful.\n${chunks}`;
				}
			} catch (e) {
				console.warn('rag search failed', e);
			}
		}

		const docBlock = aiStore.useDocContext && docText.trim()
			? `The user is editing a markdown document. Current contents follow between <doc> tags. Refer to it only when relevant.\n<doc>\n${clamp(docText, 80_000)}\n</doc>`
			: '';

		const system = [vaultBlock, docBlock].filter(Boolean).join('\n\n') || undefined;

		const toSend: ChatMessage[] = messages
			.slice(0, -1)
			.map(({ role, content }) => ({ role, content }));

		try {
			handle = await chatStream(
				aiStore.provider,
				toSend,
				{ model: aiStore.model, system },
				{
					onDelta(text) {
						const idx = messages.length - 1;
						const last = messages[idx];
						if (!last) return;
						messages[idx] = { ...last, content: last.content + text };
					},
					onDone() {
						const idx = messages.length - 1;
						const last = messages[idx];
						if (last) messages[idx] = { ...last, streaming: false };
						busy = false;
						handle = null;
					},
					onError(message) {
						const idx = messages.length - 1;
						const last = messages[idx];
						if (last) messages[idx] = { ...last, streaming: false };
						errorText = message;
						busy = false;
						handle = null;
					}
				}
			);
		} catch (e) {
			errorText = String(e);
			busy = false;
		}
	}

	async function stop() {
		await handle?.stop();
		handle = null;
	}

	function reset() {
		if (busy) return;
		messages = [];
		errorText = null;
	}

	function onKey(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			void send();
		}
	}
</script>

<div class="flex h-full flex-col bg-neutral-50 dark:bg-neutral-900">
	<div
		class="flex items-center justify-between border-b border-neutral-200 px-3 py-2 dark:border-neutral-800"
	>
		<div class="flex items-center gap-2">
			<span class="text-xs font-semibold uppercase tracking-wider text-neutral-500">Assistant</span>
			<button
				class="rounded border border-neutral-300 px-2 py-0.5 text-[11px] hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800"
				onclick={onOpenSettings}
			>
				{info.label} · {aiStore.model}
			</button>
		</div>
		<div class="flex items-center gap-2">
			<label class="flex cursor-pointer items-center gap-1 text-[11px] text-neutral-500">
				<input
					type="checkbox"
					class="h-3 w-3"
					checked={aiStore.useDocContext}
					onchange={(e) => aiStore.setDocContext((e.currentTarget as HTMLInputElement).checked)}
				/>
				doc context
			</label>
			{#if ragStore.enabled}
				<label class="flex cursor-pointer items-center gap-1 text-[11px] text-neutral-500">
					<input
						type="checkbox"
						class="h-3 w-3"
						checked={ragStore.useVaultContext}
						onchange={(e) =>
							ragStore.setUseVaultContext((e.currentTarget as HTMLInputElement).checked)}
					/>
					vault
				</label>
			{/if}
			<button
				class="text-[11px] text-neutral-500 hover:text-neutral-800 dark:hover:text-neutral-200"
				onclick={reset}
				disabled={busy}
			>
				clear
			</button>
		</div>
	</div>

	<div bind:this={scrollHost} class="flex-1 space-y-3 overflow-auto p-3 text-sm">
		{#if messages.length === 0}
			<p class="text-neutral-500">
				Ask a question about your document, or anything else. {aiStore.useDocContext
					? 'The current doc is attached as context.'
					: 'Doc context is off.'}
			</p>
		{/if}
		{#each messages as m, i (i)}
			<div
				class="rounded-md p-2 {m.role === 'user'
					? 'bg-neutral-200 dark:bg-neutral-800'
					: 'bg-white dark:bg-neutral-950 border border-neutral-200 dark:border-neutral-800'}"
			>
				<div class="mb-1 text-[10px] uppercase tracking-wider text-neutral-400">
					{m.role}{m.streaming ? ' · streaming' : ''}
				</div>
				{#if m.role === 'assistant'}
					<div class="chat-markdown prose prose-sm max-w-none break-words dark:prose-invert">
						<!-- eslint-disable-next-line svelte/no-at-html-tags -->
						{@html renderMarkdown(m.content)}{#if m.streaming}<span class="animate-pulse">▌</span>{/if}
					</div>
				{:else}
					<div class="whitespace-pre-wrap break-words text-neutral-900 dark:text-neutral-100">
						{m.content}
					</div>
				{/if}
			</div>
		{/each}
		{#if errorText}
			<div
				class="rounded-md border border-red-300 bg-red-50 p-2 text-xs text-red-700 dark:border-red-800 dark:bg-red-900/30 dark:text-red-300"
			>
				{errorText}
			</div>
		{/if}
	</div>

	<div class="border-t border-neutral-200 p-2 dark:border-neutral-800">
		<textarea
			class="h-20 w-full resize-none rounded-md border border-neutral-300 bg-white px-2 py-1 text-sm outline-none focus:border-neutral-500 dark:border-neutral-700 dark:bg-neutral-950 dark:focus:border-neutral-500"
			placeholder="Ask something… (Enter to send, Shift+Enter for newline)"
			bind:value={input}
			onkeydown={onKey}
		></textarea>
		<div class="mt-1 flex justify-end gap-2">
			{#if busy}
				<button
					class="rounded-md border border-red-300 px-3 py-1 text-sm text-red-700 hover:bg-red-50 dark:border-red-800 dark:text-red-300 dark:hover:bg-red-900/30"
					onclick={stop}
				>
					Stop
				</button>
			{:else}
				<button
					class="rounded-md bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-700 disabled:opacity-50 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
					onclick={send}
					disabled={!input.trim()}
				>
					Send
				</button>
			{/if}
		</div>
	</div>
</div>

<style>
	/* Tighten prose defaults for the small chat bubble so markdown doesn't
	   feel overly spacious. Marked output is HTML injected via {@html};
	   :global lets us style descendants that aren't in our component scope. */
	:global(.chat-markdown > *:first-child) {
		margin-top: 0;
	}
	:global(.chat-markdown > *:last-child) {
		margin-bottom: 0;
	}
	:global(.chat-markdown p),
	:global(.chat-markdown ul),
	:global(.chat-markdown ol),
	:global(.chat-markdown pre),
	:global(.chat-markdown blockquote) {
		margin: 0.5em 0;
	}
	:global(.chat-markdown li) {
		margin: 0.125em 0;
	}
	:global(.chat-markdown h1),
	:global(.chat-markdown h2),
	:global(.chat-markdown h3) {
		margin: 0.6em 0 0.3em;
	}
	:global(.chat-markdown pre) {
		padding: 0.5em;
		overflow-x: auto;
	}
</style>
