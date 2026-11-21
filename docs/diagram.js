function element(tag, text, className) {
    const node = document.createElement(tag);
    if (text !== undefined) node.textContent = text;
    if (className) node.className = className;
    return node;
}

function fields(entries) {
    const list = element('dl', undefined, 'diagram-fields');
    for (const [label, value] of entries) {
        if (value === undefined || value === null || value === '') continue;
        list.append(element('dt', label), element('dd', value));
    }
    return list;
}

function branch(title, contents) {
    const section = element('section', undefined, 'diagram-branch');
    section.append(element('h3', title), contents);
    return section;
}

function recordValue(record) {
    const value = record.value.value;
    if (value === undefined || value === null) {
        if (record.value.kind === 'manufacturer_specific') {
            return (record.data_hex?.split(' ').filter(Boolean).length || 0) + ' bytes';
        }
        return record.value.kind === 'none' ? 'No data' : record.value.kind.replaceAll('_', ' ');
    }
    const display = typeof value === 'object'
        ? Object.entries(value).map(([name, part]) => name + '=' + part).join(', ')
        : String(value);
    return record.unit ? display + ' ' + record.unit : display;
}

function recordNode(record) {
    const node = element('details', undefined, 'diagram-record');
    const summary = element('summary');
    const quantity = record.quantities?.join(', ') || record.data_coding || 'Reading';
    summary.append(element('span', '#' + record.index + ' · ' + quantity), element('strong', recordValue(record)));
    node.append(summary, fields([
        ['Function', record.function], ['Storage', record.storage_number],
        ['Tariff', record.tariff], ['Subunit', record.subunit], ['Coding', record.data_coding],
        ['Raw bytes', [record.header_hex, record.data_hex].filter(Boolean).join(' ')],
    ]));
    return node;
}

export function renderDiagram(container, decoded) {
    const diagram = element('section', undefined, 'telegram-diagram');
    diagram.setAttribute('aria-label', 'Telegram diagram');
    const root = element('div', undefined, 'diagram-root');
    const frameKind = decoded.frame.kind.replaceAll('_', ' ');
    const frameTitle = decoded.protocol === 'wired' ? 'Wired · ' + frameKind : frameKind;
    const title = decoded.protocol === 'application' ? frameTitle : frameTitle + ' frame';
    root.append(element('h2', title),
        fields([['Function', decoded.frame.function], ['Address', decoded.frame.address],
            ['Selection mask', decoded.transport?.secondary_address_mask], ['Application', decoded.application?.protocol]]));
    const branches = element('div', undefined, 'diagram-branches');
    const meter = decoded.meter?.identity;
    if (meter) {
        branches.append(branch('Meter', fields([['ID', meter.id], ['Manufacturer', meter.manufacturer_code],
            ['Type', meter.device_type], ['Version', meter.version]])));
    }
    const security = decoded.security;
    branches.append(branch('Security', fields([['Protection', security.payload_protection],
        ['Mode', security.mode], ['Decryption', security.decryption_state?.replaceAll('_', ' ')]])));
    if (decoded.records.length) {
        const readings = element('div', undefined, 'diagram-records');
        for (const record of decoded.records) readings.append(recordNode(record));
        branches.append(branch('Data records · ' + decoded.records.length, readings));
    }
    if (decoded.diagnostics.length) {
        branches.append(branch('Diagnostics', fields(decoded.diagnostics.map(item => [item.code, item.severity + ' · ' + item.message]))));
    }
    diagram.append(root, branches);
    container.replaceChildren(diagram);
}
