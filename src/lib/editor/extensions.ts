import StarterKit from '@tiptap/starter-kit';
import Link from '@tiptap/extension-link';
import Placeholder from '@tiptap/extension-placeholder';
import { TaskList } from '@tiptap/extension-task-list';
import { TaskItem } from '@tiptap/extension-task-item';
import { TableKit } from '@tiptap/extension-table';
import BubbleMenu from '@tiptap/extension-bubble-menu';
import { Markdown } from 'tiptap-markdown';
import { SlashCommands, type SlashHandler } from './slashCommands';

export type MenuElements = {
	bubble: HTMLElement | null;
	slash: SlashHandler;
};

export function buildExtensions(menu: MenuElements) {
	return [
		StarterKit.configure({
			heading: { levels: [1, 2, 3, 4, 5, 6] }
		}),
		Link.configure({
			openOnClick: false,
			autolink: true,
			HTMLAttributes: { rel: 'noopener noreferrer', target: '_blank' }
		}),
		TaskList,
		TaskItem.configure({ nested: true }),
		TableKit.configure({
			table: { resizable: true }
		}),
		Placeholder.configure({
			placeholder: 'Type / for commands, or just start writing…'
		}),
		SlashCommands.configure({
			handler: menu.slash
		}),
		Markdown.configure({
			html: false,
			tightLists: true,
			bulletListMarker: '-',
			linkify: true,
			breaks: false,
			transformPastedText: true,
			transformCopiedText: true
		}),
		...(menu.bubble
			? [
					BubbleMenu.configure({
						element: menu.bubble,
						options: { placement: 'top' as const, offset: 8 },
						shouldShow: ({ editor, from, to }) => {
							if (from === to) return false;
							if (editor.isActive('codeBlock')) return false;
							return true;
						}
					})
				]
			: [])
	];
}
