import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { runInNewContext } from 'node:vm';
import {
    initSync,
    m_bus_decode,
    m_bus_render,
    m_bus_render_application,
    m_bus_decode_tls,
    m_bus_render_tls,
} from '../../docs/m_bus_parser_wasm_pack.js';
import { createTlsSessionPanel, parseSessionImport } from '../../docs/tls-session.js';

initSync({ module: await readFile(new URL('../../docs/m_bus_parser_wasm_pack_bg.wasm', import.meta.url)) });
const vectors = await readFile(new URL('../../crates/m-bus-core/tests/fixtures/mode13.rs', import.meta.url), 'utf8');
const record = vectors.match(/OPENSSL_AES128_RECORD: &str =\s*"([a-f0-9]+)"/)[1];
const frame = `2f4493157856341233037a0100ff0d01${record}`;
const session = {
    name: 'Test meter',
    cipher_suite: 'TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256',
    direction: 'client_to_server',
    key_hex: '000102030405060708090a0b0c0d0e0f',
    fixed_iv_hex: '01020304',
    sequence_number: '5',
};

class Element {
    value = '';
    checked = false;
    textContent = '';
    children = [];
    listeners = new Map();
    dataset = {};
    attributes = new Map();
    addEventListener(name, handler) { this.listeners.set(name, handler); }
    append(child) { this.children.push(child); }
    replaceChildren() { this.children = []; this.value = ''; }
    setAttribute(name, value) { this.attributes.set(name, value); }
    removeAttribute(name) { this.attributes.delete(name); }
    focus() {}
    async dispatch(name, event = { target: this }) { await this.listeners.get(name)?.(event); }
}

function document() {
    const elements = new Map();
    return {
        getElementById(id) {
            if (!elements.has(id)) elements.set(id, new Element());
            return elements.get(id);
        },
        createElement() { return new Element(); },
        defaultView: new Element(),
    };
}

async function importSession(doc, value = session) {
    const input = doc.getElementById('tls-import');
    input.files = [{ size: 512, text: async () => JSON.stringify(value) }];
    await input.dispatch('change');
}

test('compiled WASM inspects records and decrypts with a session', () => {
    const missing = m_bus_decode(frame);
    assert.equal(missing.security.decryption_state, 'session_required');
    assert.equal(missing.security.tls.records[0].length, 27);
    const decoded = m_bus_decode_tls(frame, JSON.stringify(session));
    assert.equal(decoded.security.decryption_state, 'decrypted_authenticated');
    assert.equal(decoded.records.length, 1);
    for (const format of ['json', 'yaml', 'table', 'csv', 'mermaid']) {
        assert.ok(m_bus_render_tls(frame, format, JSON.stringify(session), 100, false));
    }
    assert.equal(m_bus_decode_tls(frame, JSON.stringify(session)).security.decrypted_payload_hex, '01132A');
    assert.throws(() => m_bus_decode_tls(frame, JSON.stringify({ ...session, sequence_number: '6' })), error => error.code === 'security.authentication_failed');
    assert.throws(() => m_bus_render_tls(frame, 'xml', JSON.stringify(session), 100, false), error => error.code === 'tls.unsupported_output');
});

test('session imports preserve 64-bit counters and redact errors', () => {
    assert.equal(parseSessionImport(JSON.stringify({ ...session, sequence_number: '18446744073709551615' })).sequence_number, '18446744073709551615');
    assert.throws(() => parseSessionImport(JSON.stringify({ ...session, sequence_number: '18446744073709551616' })));
    assert.throws(() => parseSessionImport(JSON.stringify({ ...session, sequence_number: 5 })));
    for (const field of ['cipher_suite', 'direction', 'key_hex', 'fixed_iv_hex', 'sequence_number']) {
        assert.throws(() => parseSessionImport(JSON.stringify({ ...session, [field]: 'SECRET-VALUE' })), error => !error.message.includes('SECRET-VALUE'));
        assert.throws(() => m_bus_decode_tls(frame, JSON.stringify({ ...session, [field]: 'SECRET-VALUE' })), error => !error.message.includes('SECRET-VALUE'));
    }
});

test('panel imports, selects, edits, and clears sessions in memory', async () => {
    const doc = document();
    const panel = createTlsSessionPanel(doc);
    assert.equal(panel.sessionJson(), undefined);
    await importSession(doc);
    assert.equal(JSON.parse(panel.sessionJson()).key_hex, session.key_hex);
    await importSession(doc, { ...session, name: 'Second meter', sequence_number: '6' });
    const select = doc.getElementById('tls-session');
    select.value = '0';
    await select.dispatch('change');
    assert.equal(JSON.parse(panel.sessionJson()).sequence_number, '5');
    doc.getElementById('tls-counter').value = '6';
    await doc.getElementById('tls-counter').dispatch('input');
    assert.equal(select.value, '');
    assert.equal(JSON.parse(panel.sessionJson()).sequence_number, '6');
    await doc.getElementById('tls-clear').dispatch('click');
    assert.equal(panel.sessionJson(), undefined);
    assert.equal(doc.getElementById('tls-key').value, '');
    assert.equal(select.children.length, 1);
    await importSession(doc);
    await doc.defaultView.dispatch('pagehide');
    assert.equal(doc.getElementById('tls-key').value, '');
    assert.equal(panel.sessionJson(), undefined);
});

test('missing fields are shown and a failed import disables decryption', async () => {
    const doc = document();
    const panel = createTlsSessionPanel(doc);
    await importSession(doc);
    doc.getElementById('tls-key').value = '';
    assert.throws(() => panel.sessionJson(), /key_hex/);
    assert.equal(doc.getElementById('tls-panel').open, true);
    await importSession(doc, { ...session, key_hex: 'invalid' });
    assert.equal(panel.sessionJson(), undefined);
});

test('clearing a session cancels an unfinished file import', async () => {
    const doc = document();
    const panel = createTlsSessionPanel(doc);
    let resolve;
    const pending = new Promise(done => { resolve = done; });
    const input = doc.getElementById('tls-import');
    input.files = [{ size: 512, text: () => pending }];
    const importTask = input.dispatch('change');
    panel.clear();
    resolve(JSON.stringify(session));
    await importTask;
    assert.equal(panel.sessionJson(), undefined);
    assert.equal(doc.getElementById('tls-key').value, '');
});


test('page ignores a stale static key for detected TLS telegrams', async () => {
    const html = await readFile(new URL('../../docs/index.html', import.meta.url), 'utf8');
    const fallback = html.slice(html.indexOf('        function inspectTelegram('), html.indexOf('        function failureReport('));
    const handler = html.slice(html.indexOf('            async function parseInput('), html.indexOf('        }\n\n        function measureTableWidth('));
    for (const tlsActive of [true, false]) {
        const doc = document();
        doc.querySelectorAll = () => [];
        doc.getElementById('inputstring').value = frame;
        doc.getElementById('aeskey').value = 'invalid-stale-key';
        doc.getElementById('tls-enabled').checked = tlsActive;
        let rendered;
        let failure;
        await runInNewContext(`${fallback}\n${handler}\nparseInput('json', false)`, {
            document: doc,
            window: {},
            location: { origin: 'http://localhost', pathname: '/' },
            navigator: { userAgent: 'test' },
            currentFormat: 'json',
            lastFailure: null,
            lastTableWidth: null,
            renderGeneration: 0,
            hasDecoded: false,
            inspectedTelegram: m_bus_decode(frame),
            inspectCurrentTelegram() {},
            parserApi: { decode: m_bus_decode, frame: m_bus_render, application: m_bus_render_application },
            keyInput: doc.getElementById('aeskey'),
            version: () => 'test',
            tlsPanel: { sessionJson: () => tlsActive ? JSON.stringify(session) : undefined },
            m_bus_decode_tls,
            m_bus_render_tls,
            updateDownload() {},
            renderOutputError(element, error) { failure = error; },
            appendReportActions() {},
            async renderCodeViewer(element, source) { rendered = source; },
        });
        assert.equal(failure, undefined);
        assert.equal(
            JSON.parse(rendered).security.decryption_state,
            tlsActive ? 'decrypted_authenticated' : 'session_required',
        );
    }
});
