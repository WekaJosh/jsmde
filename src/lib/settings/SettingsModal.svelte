<script lang="ts">
	import { PROVIDER_IDS, PROVIDERS, type ProviderId } from '$lib/ai/providers';
	import { aiStore } from '$lib/ai/store.svelte';
	import { deleteApiKey, hasApiKey, saveApiKey } from '$lib/ai/client';
	import CloudPane from './CloudPane.svelte';

	type Props = { open: boolean; onClose: () => void };
	let { open, onClose }: Props = $props();

	type Tab = 'ai' | 'cloud';
	let tab = $state<Tab>('ai');

	let selectedProvider = $state<ProviderId>(aiStore.provider);
	let modelInput = $state(aiStore.model);
	let keyInput = $state('');
	let keyPresent = $state<Record<ProviderId, boolean>>({
		anthropic: false,
		openai: false,
		google: false,
		ollama: false
	});
	let saving = $state(false);
	let statusText = $state<string | null>(null);

	$effect(() => {
		if (!open) return;
		void refreshKeys();
	});

	$effect(() => {
		if (!open) return;
		selectedProvider = aiStore.provider;
		modelInput = aiStore.model;
		keyInput = '';
	});

	async function refreshKeys() {
		const entries = await Promise.all(
			PROVIDER_IDS.map(async (id) => [id, await hasApiKey(id).catch(() => false)] as const)
		);
		keyPresent = Object.fromEntries(entries) as Record<ProviderId, boolean>;
	}

	function pickProvider(id: ProviderId) {
		selectedProvider = id;
		if (!PROVIDERS[id].models.includes(modelInput)) {
			modelInput = PROVIDERS[id].defaultModel;
		}
		keyInput = '';
	}

	async function saveKey() {
		if (!keyInput.trim()) return;
		saving = true;
		try {
			await saveApiKey(selectedProvider, keyInput.trim());
			statusText = `Saved ${PROVIDERS[selectedProvider].label} credential.`;
			keyInput = '';
			await refreshKeys();
		} catch (e) {
			statusText = `Save failed: ${e}`;
		} finally {
			saving = false;
		}
	}

	async function removeKey() {
		saving = true;
		try {
			await deleteApiKey(selectedProvider);
			statusText = `Removed ${PROVIDERS[selectedProvider].label} credential.`;
			await refreshKeys();
		} catch (e) {
			statusText = `Delete failed: ${e}`;
		} finally {
			saving = false;
		}
	}

	function apply() {
		aiStore.setProvider(selectedProvider);
		const m = modelInput.trim();
		if (m) aiStore.setModel(m);
		onClose();
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') onClose();
	}

	const info = $derived(PROVIDERS[selectedProvider]);
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/30 backdrop-blur-sm"
		role="presentation"
		onclick={(e) => e.target === e.currentTarget && onClose()}
		onkeydown={onKeydown}
	>
		<div
			class="w-full max-w-2xl rounded-lg border border-neutral-300 bg-white shadow-xl dark:border-neutral-700 dark:bg-neutral-900"
			role="dialog"
			aria-modal="true"
		>
			<div
				class="flex items-center justify-between border-b border-neutral-200 px-4 py-3 dark:border-neutral-800"
			>
				<h2 class="text-sm font-semibold">Settings</h2>
				<div class="flex gap-1">
					<button
						class="rounded px-2 py-1 text-xs {tab === 'ai'
							? 'bg-neutral-200 dark:bg-neutral-800'
							: 'text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-800'}"
						onclick={() => (tab = 'ai')}
					>
						AI
					</button>
					<button
						class="rounded px-2 py-1 text-xs {tab === 'cloud'
							? 'bg-neutral-200 dark:bg-neutral-800'
							: 'text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-800'}"
						onclick={() => (tab = 'cloud')}
					>
						Cloud
					</button>
					<button
						class="ml-2 rounded px-2 py-1 text-sm text-neutral-500 hover:bg-neutral-100 dark:hover:bg-neutral-800"
						onclick={onClose}
						aria-label="Close">✕</button
					>
				</div>
			</div>
			{#if tab === 'ai'}
				<div class="grid grid-cols-[200px_1fr]">
					<nav class="border-r border-neutral-200 py-2 dark:border-neutral-800">
						{#each PROVIDER_IDS as id (id)}
							<button
								class="flex w-full items-center justify-between px-3 py-1.5 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-800 {selectedProvider ===
								id
									? 'bg-neutral-100 font-medium dark:bg-neutral-800'
									: ''}"
								onclick={() => pickProvider(id)}
							>
								<span>{PROVIDERS[id].label}</span>
								{#if keyPresent[id]}
									<span class="text-[10px] text-green-600 dark:text-green-400">saved</span>
								{/if}
							</button>
						{/each}
					</nav>
					<div class="space-y-4 p-4">
						<div>
							<label
								for="settings-key-input"
								class="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400"
							>
								{info.keyLabel}
							</label>
							<input
								id="settings-key-input"
								type="password"
								class="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm outline-none focus:border-neutral-500 dark:border-neutral-700 dark:bg-neutral-950"
								placeholder={info.keyPlaceholder}
								bind:value={keyInput}
							/>
							<p class="mt-1 text-[11px] text-neutral-500">
								{info.keyHelp} Stored in your OS keychain.
							</p>
							<div class="mt-2 flex gap-2">
								<button
									class="rounded-md bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-700 disabled:opacity-50 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
									onclick={saveKey}
									disabled={saving || !keyInput.trim()}
								>
									Save key
								</button>
								{#if keyPresent[selectedProvider]}
									<button
										class="rounded-md border border-red-300 px-3 py-1 text-sm text-red-700 hover:bg-red-50 dark:border-red-800 dark:text-red-300 dark:hover:bg-red-900/30"
										onclick={removeKey}
										disabled={saving}
									>
										Remove
									</button>
								{/if}
							</div>
						</div>

						<div>
							<label
								for="settings-model-input"
								class="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400"
							>
								Default model
							</label>
							<input
								id="settings-model-input"
								class="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm outline-none focus:border-neutral-500 dark:border-neutral-700 dark:bg-neutral-950"
								list="model-suggestions-{selectedProvider}"
								bind:value={modelInput}
							/>
							<datalist id="model-suggestions-{selectedProvider}">
								{#each info.models as m (m)}
									<option value={m}></option>
								{/each}
							</datalist>
						</div>

						{#if statusText}
							<div class="text-[11px] text-neutral-500">{statusText}</div>
						{/if}
					</div>
				</div>
			{:else}
				<CloudPane />
			{/if}
			<div
				class="flex justify-end gap-2 border-t border-neutral-200 px-4 py-3 dark:border-neutral-800"
			>
				<button
					class="rounded-md border border-neutral-300 px-3 py-1 text-sm hover:bg-neutral-100 dark:border-neutral-700 dark:hover:bg-neutral-800"
					onclick={onClose}>Cancel</button
				>
				<button
					class="rounded-md bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-700 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
					onclick={apply}>Save</button
				>
			</div>
		</div>
	</div>
{/if}
