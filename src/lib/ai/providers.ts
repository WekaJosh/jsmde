export type ProviderId = 'anthropic' | 'openai' | 'google' | 'ollama';

export type ProviderInfo = {
	id: ProviderId;
	label: string;
	keyLabel: string;
	keyPlaceholder: string;
	keyHelp: string;
	models: string[];
	defaultModel: string;
};

export const PROVIDERS: Record<ProviderId, ProviderInfo> = {
	anthropic: {
		id: 'anthropic',
		label: 'Anthropic (Claude)',
		keyLabel: 'API key',
		keyPlaceholder: 'sk-ant-…',
		keyHelp: 'Get a key at console.anthropic.com.',
		models: ['claude-opus-4-7', 'claude-sonnet-4-6', 'claude-haiku-4-5-20251001'],
		defaultModel: 'claude-sonnet-4-6'
	},
	openai: {
		id: 'openai',
		label: 'OpenAI',
		keyLabel: 'API key',
		keyPlaceholder: 'sk-…',
		keyHelp: 'Get a key at platform.openai.com.',
		models: ['gpt-5', 'gpt-4.1', 'gpt-4o'],
		defaultModel: 'gpt-5'
	},
	google: {
		id: 'google',
		label: 'Google (Gemini)',
		keyLabel: 'API key',
		keyPlaceholder: 'AIza…',
		keyHelp: 'Get a key at aistudio.google.com.',
		models: ['gemini-2.5-pro', 'gemini-2.5-flash'],
		defaultModel: 'gemini-2.5-flash'
	},
	ollama: {
		id: 'ollama',
		label: 'Ollama (local)',
		keyLabel: 'Base URL',
		keyPlaceholder: 'http://127.0.0.1:11434',
		keyHelp: 'Leave blank to use the default localhost.',
		models: ['llama3.2', 'qwen2.5', 'mistral'],
		defaultModel: 'llama3.2'
	}
};

export const PROVIDER_IDS: ProviderId[] = ['anthropic', 'openai', 'google', 'ollama'];
