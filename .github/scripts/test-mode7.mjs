import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import vm from 'node:vm';

const site = new URL('../../docs/', import.meta.url);
const html = await readFile(new URL('index.html', site), 'utf8');
const source = await readFile(new URL('m_bus_parser_wasm_pack.js', site), 'utf8');
const wasm = await import(`data:text/javascript;base64,${Buffer.from(source).toString('base64')}`);
wasm.initSync({ module: await readFile(new URL('m_bus_parser_wasm_pack_bg.wasm', site)) });

const exampleSource = html.match(/const telegramExamples = ([\s\S]*?);\n/)[1];
const functions = ['errorDetails', 'inspectTelegram', 'renderWithFallback', 'securityPresentation']
    .map(name => html.match(new RegExp(`function ${name}\\([\\s\\S]*?(?=\\n        function )`))[0]);
const page = vm.runInNewContext(`${functions.join('\n')}\n({ examples: ${exampleSource}, renderWithFallback, securityPresentation })`);
const api = { decode: wasm.m_bus_decode, frame: wasm.m_bus_render, application: wasm.m_bus_render_application };
const example = page.examples[7];

for (const [protocol, count] of [['wired', 2], ['wireless', 3]]) {
    test(`Mode 7 ${protocol} example authenticates with its supplied master key`, () => {
        assert.match(example.key, /^[0-9A-F]{32}$/);
        for (const format of ['json', 'yaml', 'table', 'csv', 'mermaid', 'xml', 'annotated']) {
            const result = page.renderWithFallback(api, example.telegrams[protocol], format, example.key, 100);
            assert.equal(result.mode, protocol);
            assert.equal(result.security.decryption_state, 'decrypted_authenticated');
            assert.equal(result.security.key_id, 0);
            assert.equal(result.security.message_counter, 2739);
            assert.equal(result.decoded.records.length, count);
            assert.equal(result.decoded.records[0].value.value, '28504.27');
            assert.equal(page.securityPresentation(result.decoded).status, 'Decrypted and authenticated');
            assert.ok(result.output.length > 0);
        }
    });

    test(`Mode 7 ${protocol} example refuses a wrong key`, () => {
        assert.throws(
            () => page.renderWithFallback(api, example.telegrams[protocol], 'json', '0'.repeat(32), 100),
            error => error.code === 'security.authentication_failed',
        );
        const inspected = api.decode(example.telegrams[protocol], undefined, true);
        assert.equal(inspected.security.decryption_state, 'key_missing');
        assert.equal(inspected.records.length, 0);
        assert.equal(page.securityPresentation(inspected, { code: 'security.authentication_failed' }).status, 'Verification failed');
    });
}
