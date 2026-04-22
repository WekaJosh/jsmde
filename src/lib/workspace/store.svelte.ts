import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { DirEntry } from '$lib/tauri';

const WORKSPACE_KEY = 'jsmde:workspace';
const OPEN_FILE_KEY = 'jsmde:open-file';

type WorkspaceChangeEvent = { kind: string; paths: string[] };

function loadString(key: string): string | null {
	try {
		return localStorage.getItem(key);
	} catch {
		return null;
	}
}

function saveString(key: string, value: string | null) {
	try {
		if (value === null) localStorage.removeItem(key);
		else localStorage.setItem(key, value);
	} catch {
		/* ignore */
	}
}

class WorkspaceStore {
	root = $state<string | null>(loadString(WORKSPACE_KEY));
	openPath = $state<string | null>(loadString(OPEN_FILE_KEY));
	entries = $state<DirEntry[]>([]);
	changeTick = $state(0);

	#unlisten: UnlistenFn | null = null;

	async setRoot(path: string | null) {
		if (this.root && this.root !== path) {
			await invoke('unwatch_workspace').catch(() => {});
		}
		this.root = path;
		saveString(WORKSPACE_KEY, path);
		if (path) {
			await invoke('watch_workspace', { path });
			await this.refresh();
		} else {
			this.entries = [];
		}
	}

	async setOpenPath(path: string | null) {
		this.openPath = path;
		saveString(OPEN_FILE_KEY, path);
	}

	async refresh() {
		if (!this.root) return;
		try {
			this.entries = await invoke<DirEntry[]>('list_dir', { path: this.root });
		} catch {
			this.entries = [];
		}
	}

	async init() {
		this.#unlisten = await listen<WorkspaceChangeEvent>('workspace://change', () => {
			this.changeTick++;
			void this.refresh();
		});
		if (this.root) {
			await invoke('watch_workspace', { path: this.root }).catch(() => {});
			await this.refresh();
		}
	}

	teardown() {
		this.#unlisten?.();
		this.#unlisten = null;
	}
}

export const workspace = new WorkspaceStore();
