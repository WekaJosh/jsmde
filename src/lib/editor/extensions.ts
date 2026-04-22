import StarterKit from '@tiptap/starter-kit';
import Link from '@tiptap/extension-link';
import Placeholder from '@tiptap/extension-placeholder';
import { TaskList } from '@tiptap/extension-task-list';
import { TaskItem } from '@tiptap/extension-task-item';
import { TableKit } from '@tiptap/extension-table';
import { Markdown } from 'tiptap-markdown';

export function buildExtensions() {
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
			placeholder: 'Start writing…'
		}),
		Markdown.configure({
			html: false,
			tightLists: true,
			bulletListMarker: '-',
			linkify: true,
			breaks: false,
			transformPastedText: true,
			transformCopiedText: true
		})
	];
}
