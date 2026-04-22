import { invoke } from '@tauri-apps/api/core';

export type DirEntry = {
	name: string;
	path: string;
	is_dir: boolean;
	size: number | null;
	modified_ms: number | null;
};

export const readFile = (path: string) => invoke<string>('read_file', { path });

export const writeFile = (path: string, contents: string) =>
	invoke<void>('write_file', { path, contents });

export const listDir = (path: string) => invoke<DirEntry[]>('list_dir', { path });
