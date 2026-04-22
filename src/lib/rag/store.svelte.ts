import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
	ragReindexFile,
	ragSetEnabled,
	ragStatus,
	type IndexProgress,
	type RagStatus
} from './client';
import { workspace } from '$lib/workspace/store.svelte';

type WorkspaceChangeEvent = { kind: string; paths: string[] };

const MD_EXT = /\.(md|markdown|mdx)$/i;

function toRel(abs: string, root: string): string | null {
	const normRoot = root.replace(/[\\/]+$/, '');
	if (!abs.startsWith(normRoot)) return null;
	return abs.slice(normRoot.length + 1).replace(/\\/g, '/');
}

const STORAGE_KEY = 'jsmde:rag';

type Persisted = {
	useVaultContext: boolean;
};

function load(): Persisted {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (raw) {
			const parsed = JSON.parse(raw) as Partial<Persisted>;
			return { useVaultContext: parsed.useVaultContext ?? false };
		}
	} catch {
		/* ignore */
	}
	return { useVaultContext: false };
}

function persist(s: Persisted) {
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
	} catch {
		/* ignore */
	}
}

class RagStore {
	enabled = $state(false);
	filesIndexed = $state(0);
	chunks = $state(0);
	lastIndexedMs = $state<number | null>(null);
	model = $state('');
	ollamaUrl = $state('');

	useVaultContext = $state(false);
	indexing = $state(false);
	progress = $state<IndexProgress | null>(null);

	private unlisten: UnlistenFn | null = null;
	private unlistenWs: UnlistenFn | null = null;
	private pending = new Set<string>();
	private flushTimer: ReturnType<typeof setTimeout> | null = null;

	constructor() {
		const initial = load();
		this.useVaultContext = initial.useVaultContext;
	}

	async init() {
		if (!this.unlisten) {
			this.unlisten = await listen<IndexProgress>('rag://progress', (ev) => {
				this.progress = ev.payload;
				this.indexing = ev.payload.phase !== 'done';
			});
		}
		if (!this.unlistenWs) {
			this.unlistenWs = await listen<WorkspaceChangeEvent>('workspace://change', (ev) => {
				if (!this.enabled || !workspace.root) return;
				for (const abs of ev.payload.paths) {
					if (!MD_EXT.test(abs)) continue;
					const rel = toRel(abs, workspace.root);
					if (rel) this.pending.add(rel);
				}
				if (this.pending.size > 0) this.scheduleFlush();
			});
		}
		await this.refresh(null);
	}

	private scheduleFlush() {
		if (this.flushTimer) return;
		this.flushTimer = setTimeout(() => {
			this.flushTimer = null;
			void this.flush();
		}, 1500);
	}

	private async flush() {
		if (!workspace.root || !this.enabled) {
			this.pending.clear();
			return;
		}
		const batch = [...this.pending];
		this.pending.clear();
		for (const rel of batch) {
			try {
				await ragReindexFile(workspace.root, rel);
			} catch (e) {
				console.warn('rag reindex failed', rel, e);
			}
		}
		await this.refresh(workspace.root);
	}

	async refresh(workspace: string | null) {
		try {
			const status = await ragStatus(workspace);
			this.applyStatus(status);
		} catch {
			/* ignore */
		}
	}

	async setEnabled(value: boolean) {
		await ragSetEnabled(value);
		this.enabled = value;
		if (!value) {
			this.useVaultContext = false;
			this.save();
		}
	}

	setUseVaultContext(value: boolean) {
		this.useVaultContext = value && this.enabled;
		this.save();
	}

	private applyStatus(s: RagStatus) {
		this.enabled = s.enabled;
		this.filesIndexed = s.files_indexed;
		this.chunks = s.chunks;
		this.lastIndexedMs = s.last_indexed_ms;
		this.model = s.model;
		this.ollamaUrl = s.ollama_url;
		if (!s.enabled && this.useVaultContext) {
			this.useVaultContext = false;
			this.save();
		}
	}

	private save() {
		persist({ useVaultContext: this.useVaultContext });
	}
}

export const ragStore = new RagStore();
