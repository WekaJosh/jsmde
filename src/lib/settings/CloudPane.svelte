<script lang="ts">
	import { syncStore } from '$lib/sync/store.svelte';
	import {
		gdriveSaveClientId,
		gdriveSignIn,
		gdriveSignOut
	} from '$lib/sync/client';

	let clientIdInput = $state('');
	let busy = $state(false);
	let statusText = $state<string | null>(null);

	async function saveClientId() {
		const value = clientIdInput.trim();
		if (!value) return;
		busy = true;
		try {
			await gdriveSaveClientId(value);
			statusText = 'Client ID saved.';
			clientIdInput = '';
			await syncStore.refresh();
		} catch (e) {
			statusText = `Save failed: ${e}`;
		} finally {
			busy = false;
		}
	}

	async function signIn() {
		busy = true;
		statusText = 'Opening browser — complete the Google sign-in, then return here.';
		try {
			await gdriveSignIn();
			await syncStore.refresh();
			statusText = 'Signed in.';
		} catch (e) {
			statusText = `Sign-in failed: ${e}`;
		} finally {
			busy = false;
		}
	}

	async function signOut() {
		busy = true;
		try {
			await gdriveSignOut();
			await syncStore.refresh();
			statusText = 'Signed out. Local files are untouched.';
		} catch (e) {
			statusText = `Sign-out failed: ${e}`;
		} finally {
			busy = false;
		}
	}
</script>

<div class="space-y-4 p-4">
	<div>
		<h3 class="mb-1 text-sm font-semibold">Google Drive</h3>
		<p class="text-[11px] text-neutral-500">
			Create an OAuth 2.0 Client ID of type "Desktop app" in Google Cloud Console with the Drive
			API enabled, then paste the client ID below. The client secret is not required (PKCE is
			used). Only a dedicated <code>jsmde</code> folder in your Drive is accessed.
		</p>
	</div>

	{#if !syncStore.gdrive.has_client_id}
		<div>
			<label
				for="gdrive-client-id"
				class="mb-1 block text-xs font-medium text-neutral-600 dark:text-neutral-400"
			>
				Client ID
			</label>
			<input
				id="gdrive-client-id"
				class="w-full rounded border border-neutral-300 bg-white px-2 py-1 text-sm outline-none focus:border-neutral-500 dark:border-neutral-700 dark:bg-neutral-950"
				placeholder="xxxxxxxxxxxx.apps.googleusercontent.com"
				bind:value={clientIdInput}
			/>
			<button
				class="mt-2 rounded-md bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-700 disabled:opacity-50 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
				onclick={saveClientId}
				disabled={busy || !clientIdInput.trim()}
			>
				Save client ID
			</button>
		</div>
	{:else}
		<div class="flex items-center gap-2 text-sm">
			<span
				class="inline-block h-2 w-2 rounded-full {syncStore.gdrive.signed_in
					? 'bg-green-500'
					: 'bg-neutral-400'}"
			></span>
			<span>
				Client ID configured · {syncStore.gdrive.signed_in ? 'signed in' : 'not signed in'}
			</span>
		</div>
		<div class="flex gap-2">
			{#if !syncStore.gdrive.signed_in}
				<button
					class="rounded-md bg-neutral-900 px-3 py-1 text-sm text-white hover:bg-neutral-700 disabled:opacity-50 dark:bg-white dark:text-neutral-900 dark:hover:bg-neutral-200"
					onclick={signIn}
					disabled={busy}
				>
					Sign in with Google
				</button>
			{:else}
				<button
					class="rounded-md border border-red-300 px-3 py-1 text-sm text-red-700 hover:bg-red-50 dark:border-red-800 dark:text-red-300 dark:hover:bg-red-900/30"
					onclick={signOut}
					disabled={busy}
				>
					Sign out
				</button>
			{/if}
		</div>
	{/if}

	{#if statusText}
		<div class="text-[11px] text-neutral-500">{statusText}</div>
	{/if}
</div>
