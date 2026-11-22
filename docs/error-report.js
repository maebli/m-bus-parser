const issuePage = 'https://github.com/maebli/m-bus-parser/issues/new';
const parserPage = 'https://maebli.github.io/m-bus-parser/';
const attachmentName = 'm-bus-parser-report.txt';

export function prepareErrorReport(attempt, product, useIssueForm = true) {
    const decoded = attempt.decoded;
    const meter = decoded?.meter?.identity;
    const identity = [meter?.manufacturer_code, meter?.id, meter?.device_type].filter(Boolean).join(' · ');
    const reason =
        attempt.error?.code || decoded?.diagnostics?.[0]?.code || product?.name || identity || attempt.format;
    const context = [
        ['Parser version', attempt.version],
        ['View', attempt.format],
        ['Table width (columns)', attempt.tableWidth],
        ['Input encoding', attempt.encoding],
        ['Parse mode', decoded?.protocol || 'Automatic'],
        ['Meter', identity],
        ['Device version', meter?.version],
        ['Decode status', decoded?.decode_state],
        ['Security mode', decoded?.security?.mode],
        ['Decryption', decoded?.security?.decryption_state],
        ['AES key supplied', attempt.keySupplied ? 'Yes (omitted)' : 'No'],
        ['TLS session selected', attempt.tlsEnabled ? 'Yes (omitted)' : 'No'],
        ['Product', product?.name],
        ['Selected reading', product?.reading],
        ['Camera', product?.camera],
        ['Browser', attempt.userAgent]
    ]
        .filter(([, value]) => value !== undefined && value !== '')
        .map(([label, value]) => label + ': ' + value)
        .join('\n');
    const replay = new URL(parserPage);
    replay.search = new URLSearchParams({
        data: attempt.payload,
        format: attempt.format,
        encoding: attempt.encoding
    });
    const diagnostics = [...(decoded?.diagnostics || [])];
    if (attempt.error) diagnostics.push(attempt.error);
    const fields = [
        ['context', 'Parser and meter details', context],
        ['telegram', 'Telegram as entered', attempt.payload],
        ['replay', 'Reproduction link', attempt.payload ? replay.href : ''],
        ['diagnostics', 'Diagnostics', diagnostics.length ? JSON.stringify(diagnostics, null, 2) : 'None reported'],
        ['output', 'Observed output', attempt.output || 'No output']
    ];
    const url = new URL(issuePage);
    url.searchParams.set('title', '[Parser error] ' + reason);
    if (useIssueForm) url.searchParams.set('template', 'parser-error.yml');
    else url.searchParams.set('body', '## What went wrong?\n\nDescribe what you expected here.');
    const note = 'Attach the downloaded ' + attachmentName + ' for the full report.';
    let needsAttachment = false;
    for (const [id, label, value] of fields) {
        if (!value) continue;
        const key = useIssueForm ? id : 'body';
        const previous = url.searchParams.get(key);
        const content = id === 'replay' ? value : value.replace(/^/gm, '    ');
        url.searchParams.set(key, useIssueForm ? value : previous + '\n\n## ' + label + '\n\n' + content);
        if (url.href.length > 7300) {
            if (previous === null) url.searchParams.delete(key);
            else url.searchParams.set(key, previous);
            needsAttachment = true;
        }
    }
    if (!needsAttachment) return { url };
    const noteField = useIssueForm ? 'context' : 'body';
    url.searchParams.set(noteField, [url.searchParams.get(noteField), note].filter(Boolean).join('\n\n'));
    const attachment = fields
        .map(([id, , value]) => id.toUpperCase() + '\n' + value)
        .join('\n\n');
    return { url, attachment };
}

export function openErrorReport(attempt, product) {
    const { url, attachment } = prepareErrorReport(attempt, product, location.origin === new URL(parserPage).origin);
    if (attachment) {
        const link = document.createElement('a');
        link.href = URL.createObjectURL(new Blob([attachment], { type: 'text/plain;charset=utf-8' }));
        link.download = attachmentName;
        link.click();
        setTimeout(() => URL.revokeObjectURL(link.href), 60000);
    }
    window.open(url.href, '_blank', 'noopener,noreferrer');
}
