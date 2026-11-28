export function normalizeInput(input, format = 'auto') {
    const trimmed = input.trim();
    const hexTokens = trimmed.split(/[\t\n\f\r ,:\-]+/).filter(Boolean);
    const hexLines = trimmed.split(/\r?\n/).map(line => line.trim()).filter(Boolean);
    const packetLines = hexLines.length > 1 && hexLines.every(line => /^(?:[0-9a-f]{2})+$/i.test(line));
    const isHex = /^[0-9a-f]+$/i.test(trimmed)
        ? trimmed.length % 2 === 0
        : hexTokens.length > 0 && hexTokens.every(token => /^(?:0x)?[0-9a-f]{2}$/i.test(token));
    if (format === 'hex' || format === 'auto' && (isHex || packetLines)) return input;
    const compact = input.replace(/[\t\n\f\r ]/g, '');
    try {
        const decoded = atob(compact);
        const canonical = btoa(decoded);
        if (compact !== canonical && compact !== canonical.replace(/=+$/, '')) {
            throw new Error('Invalid Base64 encoding.');
        }
        return Array.from(decoded, byte => byte.charCodeAt(0).toString(16).padStart(2, '0')).join('');
    } catch {
        if (format === 'auto') return input;
        const error = new Error('Enter valid Base64 using A–Z, a–z, 0–9, + and /, with optional = padding.');
        error.code = 'input.invalid_base64';
        error.layer = 'input';
        throw error;
    }
}
