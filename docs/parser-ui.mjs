const REPOSITORY = 'https://github.com/maebli/m-bus-parser';

export function errorDetails(error) {
    return {
        code: error?.code || 'render.failed',
        layer: error?.layer || 'render',
        message: String(error?.message || error),
        ...(error?.byteOffset !== undefined ? { byteOffset: error.byteOffset } : {}),
    };
}

// Decide fallback eligibility using the decoder, independently of output format.
// A format/decryption failure on an identified frame must not reinterpret bytes.
export function renderWithFallback(api, input, format, key, width) {
    let decoded;
    try {
        decoded = api.decode(input, key, true);
    } catch (error) {
        if (error.code !== 'frame.invalid') throw error;
        try {
            return {
                output: api.application(input, format, key, width, true),
                mode: 'application',
                diagnostics: [],
            };
        } catch (applicationError) {
            const failure = new Error('Full-frame and application-only parsing failed.');
            failure.code = 'parse.failed';
            failure.layer = 'application';
            failure.attempts = [
                { mode: 'wired / wireless frame', ...errorDetails(error) },
                { mode: 'application layer / DIF-VIF records', ...errorDetails(applicationError) },
            ];
            throw failure;
        }
    }
    return {
        output: api.frame(input, format, key, width, true),
        mode: decoded.protocol,
        diagnostics: decoded.diagnostics.filter(item => item.severity === 'warning' || item.severity === 'error'),
    };
}

export function failureReport(attempt) {
    // Build from the captured attempt, never the current page URL or key field.
    const replay = new URL(attempt.page);
    replay.search = new URLSearchParams({ data: attempt.payload, format: attempt.format });
    replay.hash = '';
    const diagnostics = attempt.error?.attempts || [errorDetails(attempt.error)];
    const indented = value => String(value).split('\n').map(line => `    ${line}`).join('\n');
    return [
        '### Parser failure',
        `Parser version: ${attempt.version}`,
        `Output format: ${attempt.format}`,
        `Parse mode: ${attempt.mode || 'automatic (wired, wireless, then application-only)'}`,
        `AES key supplied: ${attempt.keySupplied ? 'yes (omitted)' : 'no'}`,
        `Browser: ${attempt.userAgent}`,
        '', '### Error and parse attempts', '',
        indented(JSON.stringify(diagnostics, null, 2)),
        '', '### Payload', '', indented(attempt.payload),
        '', '### Reproduce', '', replay.href,
        '', '### Meter / expected result', '',
        'Meter manufacturer and model, expected readings, and any relevant protocol documentation:',
    ].join('\n');
}

export function issueLink(attempt) {
    const title = `Parser failure: ${attempt.error?.code || 'parse.failed'}`;
    const body = failureReport(attempt);
    const issue = new URL(`${REPOSITORY}/issues/new`);
    issue.search = new URLSearchParams({ title, body });
    return issue.href;
}

export function appendReportActions(container, attempt) {
    const href = issueLink(attempt);
    const actions = document.createElement('p');
    actions.className = 'parse-report-actions';
    const issue = document.createElement('a');
    issue.href = href;
    issue.textContent = 'Open GitHub issue';
    issue.target = '_blank';
    issue.rel = 'noopener noreferrer';
    actions.append(issue);
    const help = document.createElement('p');
    help.className = 'key-help';
    help.textContent = 'Includes the error, payload, and reproduction details for review before submitting. AES keys are omitted.';
    container.append(actions, help);
}
