export function validationError(error, telegram) {
    const attempts = error.attempts || [error];
    const findings = attempts.map(attempt => {
        const authentication = ['security.authentication_failed', 'security.application_authentication_failed', 'security.lpwan_authentication_failed'].includes(attempt.code);
        const group = attempt.layer === 'security' ? 'Security' : attempt.layer === 'application' ? 'Payload' : 'M-Bus';
        const rule = authentication ? 'Message authentication' : attempt.mode || 'Telegram decoding';
        const rejected = authentication || ['frame.invalid', 'fragment.invalid', 'application.invalid', 'application.records_invalid', 'application.payload_invalid', 'parse.failed'].includes(attempt.code);
        return {
            group, rule, status: rejected ? 'failed' : 'could_not_check', detail: attempt.message,
            ...(attempt.byteOffset != null ? { offset_start: attempt.byteOffset } : {}),
        };
    });
    const assessed = findings.filter(row => row.status === 'failed').length;
    return { telegram, passed: 0, assessed, unchecked: findings.length - assessed, findings };
}

export function exportValidation(report, format) {
    if (format === 'oms-csv') {
        const columns = ['telegram', 'passed', 'assessed', 'unchecked', 'group', 'rule', 'status', 'detail', 'offset_start', 'record_number', 'reference', 'source_url'];
        const quote = value => '"' + String(value ?? '').replaceAll('"', '""') + '"';
        const rows = report.findings.map(row => columns.map(key => quote(row[key] ?? report[key])).join(','));
        return [columns.join(','), ...rows].join('\r\n') + '\r\n';
    }
    if (format === 'oms-xml') {
        const xml = document.implementation.createDocument(null, 'validation', null);
        function append(parent, name, value) {
            if (value == null) return;
            const element = xml.createElement(name);
            parent.appendChild(element);
            if (Array.isArray(value)) {
                for (const entry of value) append(element, 'finding', entry);
            } else if (typeof value === 'object') {
                for (const [key, entry] of Object.entries(value)) append(element, key, entry);
            } else {
                element.textContent = String(value);
            }
        }
        for (const [name, value] of Object.entries(report)) append(xml.documentElement, name, value);
        return new XMLSerializer().serializeToString(xml);
    }
    return JSON.stringify(report, null, 2);
}
