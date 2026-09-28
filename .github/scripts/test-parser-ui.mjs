import assert from 'node:assert/strict';
import { test } from 'node:test';
import { renderWithFallback, issueLink, appendReportActions } from '../../docs/parser-ui.mjs';

const invalidFrame = Object.assign(new Error('wired: checksum; wireless: length'), { code: 'frame.invalid', layer: 'link' });

test('application parsing is the final attempt for every output format', () => {
    for (const format of ['json', 'yaml', 'table', 'csv', 'mermaid', 'xml', 'hexview']) {
        const calls = [];
        const result = renderWithFallback({
            decode() { calls.push('frame'); throw invalidFrame; },
            frame() { assert.fail('unidentified frame must not render'); },
            application(input, outputFormat) { calls.push('application'); assert.equal(outputFormat, format); return 'decoded'; },
        }, '0313153100', format);
        assert.deepEqual(calls, ['frame', 'application']);
        assert.equal(result.mode, 'application');
    }
});

test('valid frames retain priority and partial diagnostics', () => {
    const warning = { severity: 'warning', code: 'application.record_partial', message: 'invalid record' };
    const result = renderWithFallback({
        decode: () => ({ protocol: 'wired', diagnostics: [warning] }),
        frame: () => 'frame output',
        application: () => assert.fail('must not fallback'),
    }, 'frame', 'json');
    assert.equal(result.output, 'frame output');
    assert.deepEqual(result.diagnostics, [warning]);
});

test('input, key, decryption, and rendering errors never trigger reinterpretation', () => {
    for (const code of ['input.empty', 'input.invalid_hex', 'option.invalid', 'security.failed']) {
        assert.throws(() => renderWithFallback({
            decode() { throw Object.assign(new Error(code), { code }); },
            application: () => assert.fail('must not fallback'),
        }, 'input', 'json'), { message: code });
    }
    assert.throws(() => renderWithFallback({
        decode: () => ({ protocol: 'wired', diagnostics: [] }),
        frame() { throw new Error('unsupported XML'); },
        application: () => assert.fail('must not fallback'),
    }, 'input', 'xml'), /unsupported XML/);
});

test('reports capture both attempts, exact payload, reproduction settings and no URL secrets', () => {
    let error;
    try {
        renderWithFallback({
            decode() { throw invalidFrame; },
            application() { throw Object.assign(new Error('bad record'), { code: 'application.records_invalid', byteOffset: 0 }); },
        }, '03 13', 'table');
    } catch (caught) { error = caught; }
    const payload = '03 13\n<script> & # ? ```';
    const href = issueLink({
        payload, format: 'table', version: '0.5.2', keySupplied: true,
        page: 'https://example.test/parser?key=SECRET#SECRET', userAgent: 'Test browser', error,
    });
    {
        const url = new URL(href);
        assert.equal(url.pathname, '/maebli/m-bus-parser/issues/new');
        const body = url.searchParams.get('body');
        assert.match(body, /frame.invalid/);
        assert.match(body, /application.records_invalid/);
        assert.match(body, /"byteOffset": 0/);
        assert.match(body, /Parser version: 0.5.2/);
        assert.match(body, /AES key supplied: yes \(omitted\)/);
        assert.ok(!body.includes('SECRET'));
        const replay = new URL(body.split('\n').find(line => line.startsWith('https://example.test')));
        assert.equal(replay.searchParams.get('data'), payload);
        assert.equal(replay.searchParams.get('format'), 'table');
        assert.equal(replay.hash, '');
    }
});

test('failure actions expose one safe, prefilled GitHub issue link', () => {
    const node = tag => ({ tag, children: [], append(...children) { this.children.push(...children); } });
    globalThis.document = { createElement: node, createTextNode: text => ({ text }) };
    const container = node('div');
    appendReportActions(container, { payload: 'FF', format: 'json', page: 'https://example.test', error: invalidFrame });
    const links = container.children[0].children.filter(child => child.tag === 'a');
    assert.equal(links.length, 1);
    assert.equal(links[0].textContent, 'Open GitHub issue');
    for (const link of links) {
        assert.equal(new URL(link.href).hostname, 'github.com');
        assert.equal(link.rel, 'noopener noreferrer');
        assert.equal(link.target, '_blank');
    }
    delete globalThis.document;
});

test('page replay initializes, failures capture their input, and recovery clears stale errors', async () => {
    const { readFileSync } = await import('node:fs');
    const { runInNewContext } = await import('node:vm');
    const html = readFileSync(new URL('../../docs/index.html', import.meta.url), 'utf8');
    const script = html.match(/<script type="module">([\s\S]*?)<\/script>/)[1]
        .replace(/^\s*import .*;$/gm, '')
        .replaceAll('import.meta.url', '"http://example.test/index.html"')
        .replace('setup(); // Set up', 'globalThis.ready = setup(); // Set up');
    const element = () => ({
        value: '', children: [], listeners: {}, classList: { add() {}, remove() {}, contains() { return false; } },
        append(...children) { this.children.push(...children); },
        appendChild(child) { this.children.push(child); },
        prepend(child) { this.children.unshift(child); },
        replaceChildren(...children) { this.children = children; },
        setAttribute() {}, addEventListener(name, handler) { this.listeners[name] = handler; },
    });
    const elements = new Map();
    const document = {
        body: element(), createElement: element,
        createTextNode: text => ({ text }),
        getElementById(id) { if (!elements.has(id)) elements.set(id, element()); return elements.get(id); },
    };
    const opened = [];
    const location = { origin: 'http://example.test', pathname: '/', search: '?data=0313153100&format=json' };
    const context = {
        document, location, navigator: { userAgent: 'Test browser' },
        window: { location, matchMedia: () => ({ matches: false }), open: url => opened.push(url) },
        localStorage: { getItem: () => null }, history: { replaceState() {} },
        URL, URLSearchParams, Blob, setTimeout, clearTimeout, console,
        ResizeObserver: class { observe() {} },
        mermaid: { initialize() {} }, init: async () => {}, version: () => 'test',
        m_bus_highlight: source => source,
        m_bus_decode() { throw invalidFrame; }, m_bus_render() { assert.fail('frame render'); },
        m_bus_render_application(input) { if (input === 'bad') throw new Error('bad record'); return '{"protocol":"application"}'; },
        renderWithFallback, appendReportActions, issueLink,
    };
    globalThis.document = document;
    try {
        runInNewContext(script, context);
        await context.ready;
        await new Promise(setImmediate);
        assert.equal(context.window._lastRawOutput, '{"protocol":"application"}');
        const input = document.getElementById('inputstring');
        input.value = 'bad';
        document.getElementById('parse_json').listeners.click();
        await new Promise(setImmediate);
        assert.equal(context.window._lastRawOutput, '');
        input.value = 'edited after failure';
        document.getElementById('report_issue').listeners.click();
        const report = new URL(opened[0]).searchParams.get('body');
        assert.match(report, /bad record/);
        assert.match(report, /    bad/);
        assert.ok(!report.includes('edited after failure'));
        input.value = '0313153100';
        document.getElementById('parse_json').listeners.click();
        await new Promise(setImmediate);
        assert.equal(context.window._lastRawOutput, '{"protocol":"application"}');
        assert.ok(!document.getElementById('output').children.some(child => child.className === 'parse-report-actions'));
    } finally {
        delete globalThis.document;
        if (context.window._lastDownloadUrl) URL.revokeObjectURL(context.window._lastDownloadUrl);
    }
});
