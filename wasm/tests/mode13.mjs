import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import {
    initSync,
    m_bus_decode,
    m_bus_decode_tls,
    m_bus_render_tls,
} from '../../docs/parser.js';
import { createTlsSessionPanel, parseSessionImport } from '../../docs/tls-session.js';
import { frame, session } from './mode13-fixture.mjs';

initSync({ module: await readFile(new URL('../../docs/parser_bg.wasm', import.meta.url)) });

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
