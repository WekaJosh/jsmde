// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import { renderMarkdown } from './markdown';

describe('renderMarkdown', () => {
	it('renders a heading', () => {
		const html = renderMarkdown('# Hello');
		expect(html).toContain('<h1>');
		expect(html).toContain('Hello');
	});

	it('renders a GFM task list', () => {
		const html = renderMarkdown('- [x] done\n- [ ] todo');
		expect(html).toContain('checkbox');
	});

	it('renders fenced code blocks', () => {
		const html = renderMarkdown('```ts\nconst x = 1;\n```');
		expect(html).toContain('<code');
		expect(html).toContain('const x = 1;');
	});

	it('sanitizes script tags', () => {
		const html = renderMarkdown('<script>alert(1)</script>');
		expect(html).not.toContain('<script>');
	});

	it('handles nested lists', () => {
		const html = renderMarkdown('- a\n  - b\n    - c');
		expect(html.match(/<ul>/g)?.length ?? 0).toBeGreaterThanOrEqual(3);
	});
});
