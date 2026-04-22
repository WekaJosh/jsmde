import { Extension, type Editor, type Range } from '@tiptap/core';
import Suggestion, {
	type SuggestionOptions,
	type SuggestionProps
} from '@tiptap/suggestion';

export type SlashCommand = {
	id: string;
	title: string;
	description: string;
	keywords: string[];
	icon: string;
	run: (args: { editor: Editor; range: Range }) => void;
};

export const SLASH_COMMANDS: SlashCommand[] = [
	{
		id: 'h1',
		title: 'Heading 1',
		description: 'Large section title',
		keywords: ['h1', 'heading', 'title'],
		icon: 'H1',
		run: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setHeading({ level: 1 }).run()
	},
	{
		id: 'h2',
		title: 'Heading 2',
		description: 'Medium section title',
		keywords: ['h2', 'heading'],
		icon: 'H2',
		run: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setHeading({ level: 2 }).run()
	},
	{
		id: 'h3',
		title: 'Heading 3',
		description: 'Small section title',
		keywords: ['h3', 'heading'],
		icon: 'H3',
		run: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setHeading({ level: 3 }).run()
	},
	{
		id: 'bulletList',
		title: 'Bullet list',
		description: 'Unordered list',
		keywords: ['bullet', 'list', 'ul'],
		icon: '•—',
		run: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleBulletList().run()
	},
	{
		id: 'orderedList',
		title: 'Numbered list',
		description: 'Ordered list',
		keywords: ['numbered', 'list', 'ol', 'ordered'],
		icon: '1.',
		run: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleOrderedList().run()
	},
	{
		id: 'taskList',
		title: 'Task list',
		description: 'Checkboxes',
		keywords: ['task', 'todo', 'checkbox'],
		icon: '☑',
		run: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleTaskList().run()
	},
	{
		id: 'table',
		title: 'Table',
		description: '3×3 table with header row',
		keywords: ['table', 'grid'],
		icon: '⊞',
		run: ({ editor, range }) =>
			editor
				.chain()
				.focus()
				.deleteRange(range)
				.insertTable({ rows: 3, cols: 3, withHeaderRow: true })
				.run()
	},
	{
		id: 'codeBlock',
		title: 'Code block',
		description: 'Monospaced block',
		keywords: ['code', 'snippet', '```'],
		icon: '</>',
		run: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleCodeBlock().run()
	},
	{
		id: 'blockquote',
		title: 'Quote',
		description: 'Indented quotation',
		keywords: ['quote', 'blockquote', 'citation'],
		icon: '❝',
		run: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).toggleBlockquote().run()
	},
	{
		id: 'hr',
		title: 'Divider',
		description: 'Horizontal rule',
		keywords: ['divider', 'hr', 'line', 'separator'],
		icon: '―',
		run: ({ editor, range }) =>
			editor.chain().focus().deleteRange(range).setHorizontalRule().run()
	}
];

export type SlashState = {
	query: string;
	rect: DOMRect | null;
	items: SlashCommand[];
	selectedIndex: number;
};

export type SlashHandler = {
	onOpen: (state: SlashState) => void;
	onUpdate: (state: SlashState) => void;
	onClose: () => void;
	bindPick: (pick: (i: number) => void) => void;
};

export type SlashCommandsOptions = {
	handler: SlashHandler;
	suggestion: Partial<SuggestionOptions<SlashCommand>>;
};

export function filterCommands(query: string): SlashCommand[] {
	const q = query.trim().toLowerCase();
	if (!q) return SLASH_COMMANDS;
	return SLASH_COMMANDS.filter(
		(c) =>
			c.title.toLowerCase().includes(q) || c.keywords.some((k) => k.toLowerCase().includes(q))
	);
}

export const SlashCommands = Extension.create<SlashCommandsOptions>({
	name: 'slashCommands',

	addOptions() {
		return {
			handler: {
				onOpen: () => {},
				onUpdate: () => {},
				onClose: () => {},
				bindPick: () => {}
			},
			suggestion: {
				char: '/',
				startOfLine: false,
				command: ({ editor, range, props }) => {
					(props as SlashCommand).run({ editor, range });
				}
			}
		};
	},

	addProseMirrorPlugins() {
		const handler = this.options.handler;

		return [
			Suggestion<SlashCommand>({
				editor: this.editor,
				...this.options.suggestion,
				items: ({ query }) => filterCommands(query),
				render: () => {
					let selectedIndex = 0;
					let currentItems: SlashCommand[] = [];
					let currentProps: SuggestionProps<SlashCommand> | null = null;

					const pickCoords = (props: SuggestionProps<SlashCommand>): DOMRect | null =>
						typeof props.clientRect === 'function' ? props.clientRect() : null;

					const emit = (fn: (s: SlashState) => void) => {
						if (!currentProps) return;
						fn({
							query: currentProps.query,
							rect: pickCoords(currentProps),
							items: currentItems,
							selectedIndex
						});
					};

					handler.bindPick((i: number) => {
						if (!currentProps) return;
						const cmd = currentItems[i];
						if (cmd) currentProps.command(cmd);
					});

					return {
						onStart: (props) => {
							currentProps = props;
							currentItems = props.items;
							selectedIndex = 0;
							emit(handler.onOpen);
						},
						onUpdate: (props) => {
							currentProps = props;
							currentItems = props.items;
							if (selectedIndex >= currentItems.length) selectedIndex = 0;
							emit(handler.onUpdate);
						},
						onKeyDown: ({ event }) => {
							if (event.key === 'ArrowDown') {
								if (currentItems.length > 0) {
									selectedIndex = (selectedIndex + 1) % currentItems.length;
									emit(handler.onUpdate);
								}
								return true;
							}
							if (event.key === 'ArrowUp') {
								if (currentItems.length > 0) {
									selectedIndex =
										(selectedIndex - 1 + currentItems.length) % currentItems.length;
									emit(handler.onUpdate);
								}
								return true;
							}
							if (event.key === 'Enter') {
								const cmd = currentItems[selectedIndex];
								if (cmd && currentProps) {
									currentProps.command(cmd);
									return true;
								}
							}
							if (event.key === 'Escape') {
								handler.onClose();
								return true;
							}
							return false;
						},
						onExit: () => {
							currentProps = null;
							currentItems = [];
							selectedIndex = 0;
							handler.onClose();
						}
					};
				}
			})
		];
	}
});
