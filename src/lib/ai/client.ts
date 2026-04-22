import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ProviderId } from './providers';

export type ChatMessage = {
	role: 'user' | 'assistant';
	content: string;
};

export type ChatOptions = {
	model: string;
	temperature?: number;
	max_tokens?: number;
	system?: string;
};

export type StreamChunk =
	| { type: 'delta'; text: string }
	| { type: 'done' }
	| { type: 'error'; message: string };

export async function saveApiKey(provider: ProviderId, apiKey: string) {
	await invoke('ai_save_key', { provider, apiKey });
}

export async function deleteApiKey(provider: ProviderId) {
	await invoke('ai_delete_key', { provider });
}

export async function hasApiKey(provider: ProviderId): Promise<boolean> {
	return invoke<boolean>('ai_has_key', { provider });
}

function randomId() {
	return Math.random().toString(36).slice(2) + Date.now().toString(36);
}

export type ChatStreamHandle = {
	requestId: string;
	stop: () => Promise<void>;
};

export async function chatStream(
	provider: ProviderId,
	messages: ChatMessage[],
	options: ChatOptions,
	handlers: {
		onDelta: (text: string) => void;
		onDone: () => void;
		onError: (message: string) => void;
	}
): Promise<ChatStreamHandle> {
	const requestId = randomId();
	const channel = `ai://${requestId}`;
	let unlisten: UnlistenFn | null = null;
	let settled = false;

	const finish = (ok: boolean, err?: string) => {
		if (settled) return;
		settled = true;
		unlisten?.();
		unlisten = null;
		if (ok) handlers.onDone();
		else if (err) handlers.onError(err);
	};

	unlisten = await listen<StreamChunk>(channel, (evt) => {
		const chunk = evt.payload;
		if (chunk.type === 'delta') handlers.onDelta(chunk.text);
		else if (chunk.type === 'error') finish(false, chunk.message);
		else if (chunk.type === 'done') finish(true);
	});

	try {
		await invoke('ai_chat_stream', {
			requestId,
			provider,
			messages,
			options
		});
	} catch (e) {
		finish(false, String(e));
		throw e;
	}

	return {
		requestId,
		async stop() {
			try {
				await invoke('ai_cancel', { requestId });
			} catch {
				/* ignore */
			}
		}
	};
}
