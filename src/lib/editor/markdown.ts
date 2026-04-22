import type { Editor } from '@tiptap/core';
import { marked } from 'marked';
import DOMPurify from 'dompurify';

marked.setOptions({ gfm: true, breaks: false });

export function renderMarkdown(md: string): string {
	const raw = marked.parse(md, { async: false }) as string;
	return DOMPurify.sanitize(raw);
}

type MarkdownStorage = { getMarkdown: () => string };

export function toMarkdown(editor: Editor): string {
	const md = (editor.storage as unknown as { markdown?: MarkdownStorage }).markdown;
	if (!md) {
		throw new Error('Markdown extension not loaded on editor');
	}
	return md.getMarkdown();
}

export function fromMarkdown(editor: Editor, md: string): void {
	editor.commands.setContent(md, { emitUpdate: false });
}
