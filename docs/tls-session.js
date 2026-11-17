export function parseSessionImport(text) {
    let session;
    try {
        session = JSON.parse(text);
    } catch {
        throw new Error('Session import must contain valid JSON.');
    }
    if (!session || typeof session !== 'object' || Array.isArray(session)) {
        throw new Error('Session import must contain one JSON object.');
    }
    for (const field of ['cipher_suite', 'direction', 'key_hex', 'fixed_iv_hex', 'sequence_number']) {
        if (typeof session[field] !== 'string' || !session[field].trim()) {
            throw new Error(`Missing or invalid ${field}. Provide a string.`);
        }
    }
    const keyLength = {
        TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256: 32,
        TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384: 64,
    }[session.cipher_suite];
    if (!keyLength) throw new Error('Unsupported cipher_suite. Select ECDHE-ECDSA AES-GCM.');
    if (!['client_to_server', 'server_to_client'].includes(session.direction)) {
        throw new Error('Invalid direction. Use client_to_server or server_to_client.');
    }
    if (!/^[0-9a-f]+$/i.test(session.key_hex) || session.key_hex.length !== keyLength) {
        throw new Error('Invalid key_hex. The key length must match the cipher suite.');
    }
    if (!/^[0-9a-f]{8}$/i.test(session.fixed_iv_hex)) {
        throw new Error('Invalid fixed_iv_hex. Provide eight hexadecimal digits.');
    }
    if (!/^\d+$/.test(session.sequence_number) || BigInt(session.sequence_number) > 18446744073709551615n) {
        throw new Error('Invalid sequence_number. Provide an unsigned 64-bit decimal string.');
    }
    return {
        name: typeof session.name === 'string' ? session.name : 'Imported session',
        cipher_suite: session.cipher_suite,
        direction: session.direction,
        key_hex: session.key_hex,
        fixed_iv_hex: session.fixed_iv_hex,
        sequence_number: session.sequence_number,
    };
}

export function createTlsSessionPanel(document, onChange = () => {}) {
    const element = id => document.getElementById(`tls-${id}`);
    const sessions = [];
    const fields = {
        cipher_suite: element('cipher'),
        direction: element('direction'),
        key_hex: element('key'),
        fixed_iv_hex: element('iv'),
        sequence_number: element('counter'),
    };
    const show = session => {
        for (const [name, input] of Object.entries(fields)) input.value = session[name];
        element('enabled').checked = true;
        element('status').textContent = 'Session loaded in memory. The counter stays fixed for repeated decoding.';
        onChange();
    };
    let importGeneration = 0;
    element('import').addEventListener('change', async event => {
        const generation = ++importGeneration;
        const file = event.target.files[0];
        if (!file) return;
        element('enabled').checked = false;
        onChange();
        try {
            if (file.size > 16_384) throw new Error('Session JSON must be smaller than 16 KB.');
            const text = await file.text();
            if (generation !== importGeneration) return;
            const session = parseSessionImport(text);
            sessions.push(session);
            const option = document.createElement('option');
            option.value = String(sessions.length - 1);
            option.textContent = session.name;
            element('session').append(option);
            element('session').value = option.value;
            show(session);
        } catch (error) {
            element('enabled').checked = false;
            element('status').textContent = error.message;
            onChange();
        } finally {
            event.target.value = '';
        }
    });
    element('session').addEventListener('change', () => {
        if (element('session').value !== '') show(sessions[Number(element('session').value)]);
    });
    for (const input of Object.values(fields)) {
        input.addEventListener('input', () => {
            element('session').value = '';
            element('status').textContent = 'Manual session values. Decode again to apply changes.';
            onChange();
        });
    }
    element('enabled').addEventListener('change', onChange);
    const clear = () => {
        importGeneration++;
        sessions.length = 0;
        element('session').replaceChildren();
        const option = document.createElement('option');
        option.value = '';
        option.textContent = 'Manual session';
        element('session').append(option);
        fields.cipher_suite.selectedIndex = 0;
        fields.direction.selectedIndex = 0;
        fields.key_hex.value = '';
        fields.fixed_iv_hex.value = '';
        fields.sequence_number.value = '';
        element('enabled').checked = false;
        element('import').value = '';
        element('status').textContent = 'Session data cleared.';
        onChange();
    };
    element('clear').addEventListener('click', clear);
    document.defaultView?.addEventListener('pagehide', clear);
    clear();
    return {
        loadSession(session) {
            const validated = parseSessionImport(JSON.stringify(session));
            importGeneration++;
            element('session').value = '';
            show(validated);
        },
        sessionJson() {
            if (!element('enabled').checked) return undefined;
            const session = Object.fromEntries(Object.entries(fields).map(([name, input]) => [name, input.value.trim()]));
            try {
                const validated = parseSessionImport(JSON.stringify(session));
                element('status').textContent = 'Session ready. The counter stays fixed for repeated decoding.';
                return JSON.stringify(validated);
            } catch (error) {
                element('status').textContent = error.message;
                element('panel').open = true;
                error.code = 'option.invalid';
                error.layer = 'security';
                throw error;
            }
        },
        clear,
    };
}
