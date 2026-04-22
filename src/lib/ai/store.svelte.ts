import { PROVIDERS, type ProviderId } from './providers';

const STORAGE_KEY = 'jsmde:ai';

type Persisted = {
	provider: ProviderId;
	model: string;
	useDocContext: boolean;
};

function load(): Persisted {
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (raw) {
			const parsed = JSON.parse(raw) as Partial<Persisted>;
			const provider = (parsed.provider ?? 'anthropic') as ProviderId;
			const info = PROVIDERS[provider] ?? PROVIDERS.anthropic;
			return {
				provider,
				model: parsed.model && typeof parsed.model === 'string' ? parsed.model : info.defaultModel,
				useDocContext: parsed.useDocContext ?? true
			};
		}
	} catch {
		/* ignore */
	}
	return {
		provider: 'anthropic',
		model: PROVIDERS.anthropic.defaultModel,
		useDocContext: true
	};
}

function persist(s: Persisted) {
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(s));
	} catch {
		/* ignore */
	}
}

class AiStore {
	provider = $state<ProviderId>('anthropic');
	model = $state('');
	useDocContext = $state(true);

	constructor() {
		const initial = load();
		this.provider = initial.provider;
		this.model = initial.model;
		this.useDocContext = initial.useDocContext;
	}

	setProvider(id: ProviderId) {
		this.provider = id;
		if (!PROVIDERS[id].models.includes(this.model)) {
			this.model = PROVIDERS[id].defaultModel;
		}
		this.save();
	}

	setModel(model: string) {
		this.model = model;
		this.save();
	}

	setDocContext(on: boolean) {
		this.useDocContext = on;
		this.save();
	}

	private save() {
		persist({ provider: this.provider, model: this.model, useDocContext: this.useDocContext });
	}
}

export const aiStore = new AiStore();
