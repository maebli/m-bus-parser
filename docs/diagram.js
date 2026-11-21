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

const symbols = {
    electricity: 'M13.5 2 5 13h6l-1 9 9-13h-6z',
    water: 'M12 2s-7 8.5-7 13a7 7 0 0 0 14 0c0-4.5-7-13-7-13Z',
    thermal: 'M9 14.5V5a3 3 0 0 1 6 0v9.5a5 5 0 1 1-6 0ZM12 8v9',
    gas: 'M12 2c1 6-5 7-3 12 2-1 3-3 4-5 4 3 6 6 4 10a7 7 0 0 1-12-1c-2-6 3-9 7-16Z',
    time: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM12 6v6l4 2',
    reading: 'M3 17a9 9 0 1 1 18 0ZM12 14l4-5',
};

function readingIcon(kind) {
    const ns = 'http://www.w3.org/2000/svg';
    const icon = document.createElementNS(ns, 'svg');
    icon.setAttribute('viewBox', '0 0 24 24');
    icon.setAttribute('aria-hidden', 'true');
    icon.classList.add('reading-icon');
    const path = document.createElementNS(ns, 'path');
    path.setAttribute('d', symbols[kind]);
    icon.append(path);
    return icon;
}

function meterKind(deviceType = '') {
    if (/electric/i.test(deviceType)) return 'electricity';
    if (/water/i.test(deviceType)) return 'water';
    if (/heat|cool|steam/i.test(deviceType)) return 'thermal';
    if (/gas/i.test(deviceType)) return 'gas';
    return 'reading';
}

function readingKind(quantity, medium) {
    if (/temperature/i.test(quantity)) return 'thermal';
    if (/time|date|duration/i.test(quantity)) return 'time';
    if (/voltage|current|frequency|power factor/i.test(quantity)) return 'electricity';
    if (/energy|power|volume|flow|mass/i.test(quantity)) return medium;
    return 'reading';
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

function recordNode(record, medium) {
    const node = element('details', undefined, 'diagram-record');
    const summary = element('summary');
    const quantity = record.quantities?.join(', ') || record.data_coding || 'Reading';
    const kind = readingKind(quantity, medium);
    node.dataset.kind = kind;
    const caption = element('span', undefined, 'diagram-caption');
    caption.append(element('span', '#' + record.index + ' · ' + quantity), element('strong', recordValue(record)));
    summary.append(readingIcon(kind), caption);
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
    const medium = meterKind(meter?.device_type);
    if (meter) {
        const meterBranch = branch('Meter', fields([['ID', meter.id], ['Manufacturer', meter.manufacturer_code],
            ['Type', meter.device_type], ['Version', meter.version]]));
        meterBranch.dataset.kind = medium;
        meterBranch.querySelector('h3').prepend(readingIcon(medium));
        branches.append(meterBranch);
    }
    const security = decoded.security;
    branches.append(branch('Security', fields([['Protection', security.payload_protection],
        ['Mode', security.mode], ['Decryption', security.decryption_state?.replaceAll('_', ' ')]])));
    if (decoded.records.length) {
        const readings = element('div', undefined, 'diagram-records');
        for (const record of decoded.records) readings.append(recordNode(record, medium));
        branches.append(branch('Data records · ' + decoded.records.length, readings));
    }
    if (decoded.diagnostics.length) {
        branches.append(branch('Diagnostics', fields(decoded.diagnostics.map(item => [item.code, item.severity + ' · ' + item.message]))));
    }
    diagram.append(root, branches);
    container.replaceChildren(diagram);
}
