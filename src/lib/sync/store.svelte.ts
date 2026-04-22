import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { gdriveStatus, syncRun, type GDriveStatus, type SyncReport } from './client';
import { workspace } from '$lib/workspace/store.svelte';

class SyncStore {
	gdrive = $state<GDriveStatus>({ has_client_id: false, signed_in: false, root_folder_id: null });
	running = $state(false);
	lastReport = $state<SyncReport | null>(null);
	lastRunAt = $state<number | null>(null);
	errorText = $state<string | null>(null);

	#startUnlisten: UnlistenFn | null = null;
	#doneUnlisten: UnlistenFn | null = null;

	async init() {
		await this.refresh();
		this.#startUnlisten = await listen('sync://start', () => {
			this.running = true;
		});
		this.#doneUnlisten = await listen<SyncReport>('sync://done', (e) => {
			this.running = false;
			this.lastReport = e.payload;
			this.lastRunAt = Date.now();
		});
	}

	teardown() {
		this.#startUnlisten?.();
		this.#doneUnlisten?.();
		this.#startUnlisten = null;
		this.#doneUnlisten = null;
	}

	async refresh() {
		try {
			this.gdrive = await gdriveStatus();
		} catch (e) {
			this.errorText = String(e);
		}
	}

	async runNow() {
		if (!workspace.root) {
			this.errorText = 'Open a workspace folder first.';
			return;
		}
		if (!this.gdrive.signed_in) {
			this.errorText = 'Connect Google Drive in Settings.';
			return;
		}
		this.errorText = null;
		this.running = true;
		try {
			this.lastReport = await syncRun(workspace.root);
			this.lastRunAt = Date.now();
		} catch (e) {
			this.errorText = String(e);
		} finally {
			this.running = false;
		}
	}
}

export const syncStore = new SyncStore();
