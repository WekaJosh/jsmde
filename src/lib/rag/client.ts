import { invoke } from '@tauri-apps/api/core';

export type RagStatus = {
	enabled: boolean;
	files_indexed: number;
	chunks: number;
	last_indexed_ms: number | null;
	model: string;
	ollama_url: string;
};

export type RagHit = {
	rel_path: string;
	chunk_index: number;
	content: string;
	score: number;
};

export type IndexReport = {
	files_scanned: number;
	files_indexed: number;
	chunks_written: number;
	error: string | null;
};

export type IndexProgress = {
	phase: 'scanning' | 'indexing' | 'done';
	done: number;
	total: number;
	current: string | null;
};

export const ragStatus = (workspace: string | null) =>
	invoke<RagStatus>('rag_status', { workspace });

export const ragSetEnabled = (enabled: boolean) =>
	invoke<void>('rag_set_enabled', { enabled });

export const ragReindex = (workspace: string) =>
	invoke<IndexReport>('rag_reindex', { workspace });

export const ragReindexFile = (workspace: string, relPath: string) =>
	invoke<void>('rag_reindex_file', { workspace, relPath });

export const ragSearch = (workspace: string, query: string, k = 5) =>
	invoke<RagHit[]>('rag_search', { workspace, query, k });

export const ragClear = (workspace: string) => invoke<void>('rag_clear', { workspace });
