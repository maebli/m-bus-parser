import { readFile } from 'node:fs/promises';

const vectors = await readFile(new URL('../../crates/m-bus-core/tests/fixtures/mode13.rs', import.meta.url), 'utf8');
const record = vectors.match(/OPENSSL_AES128_RECORD: &str =\s*"([a-f0-9]+)"/)[1];
export const frame = `2f4493157856341233037a0100ff0d01${record}`;
export const session = {
    name: 'Test meter',
    cipher_suite: 'TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256',
    direction: 'client_to_server',
    key_hex: '000102030405060708090a0b0c0d0e0f',
    fixed_iv_hex: '01020304',
    sequence_number: '5',
};
