import { invoke } from '@tauri-apps/api/core';

export type GDriveStatus = {
	has_client_id: boolean;
	signed_in: boolean;
	root_folder_id: string | null;
};

export type SyncReport = {
	uploaded: number;
	downloaded: number;
	conflicts: number;
	deleted_remote: number;
	errors: string[];
};

export const gdriveStatus = () => invoke<GDriveStatus>('gdrive_status');
export const gdriveSaveClientId = (clientId: string) =>
	invoke<void>('gdrive_save_client_id', { clientId });
export const gdriveSignIn = () => invoke<void>('gdrive_sign_in');
export const gdriveSignOut = () => invoke<void>('gdrive_sign_out');
export const syncRun = (workspaceRoot: string) =>
	invoke<SyncReport>('sync_run', { workspaceRoot });
