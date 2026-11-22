import { identifyModel } from './catalog.js';
import './turntable.js';

function element(tag, className, text) {
    const node = document.createElement(tag);
    node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
}

function deviceFamily(code) {
    if (code === 2) return 'electricity';
    if (code === 3) return 'gas';
    if ([4, 10, 11, 12, 13].includes(code)) return 'heat';
    if ([6, 7, 16, 21, 22, 23, 40].includes(code)) return 'water';
    if (code === 27) return 'sensor';
    return 'device';
}

function readingLabel(record) {
    const details = [record.quantities.join(' · ') || 'Reading'];
    if (record.function && record.function !== 'Instantaneous value') details.push(record.function);
    if (record.storage_number) details.push('Storage ' + record.storage_number);
    if (record.tariff) details.push('Tariff ' + record.tariff);
    if (record.subunit) details.push('Subunit ' + record.subunit);
    return details.join(' · ');
}

function primaryRecord(records, family) {
    const preference = {
        heat: ['Energy', 'Volume'], gas: ['Volume'], water: ['Volume'],
        electricity: ['Energy', 'Voltage'], sensor: ['External temperature', 'Temperature', 'Relative humidity'],
    }[family] || [];
    const measurements = records.filter(r => r.unit);
    const choices = measurements.length ? measurements : records;
    const current = choices.filter(r => !r.storage_number && r.function === 'Instantaneous value');
    for (const pool of [current, choices]) {
        for (const quantity of preference) {
            const candidates = pool.filter(r => r.quantities.includes(quantity));
            if (candidates.length) return candidates.find(r => !r.tariff && !r.subunit) || candidates[0];
        }
        if (pool.length) return pool[0];
    }
}

function displayReading(record, asset) {
    if (!record) return { value: '—', unit: '' };
    const value = String(record.value.value);
    if (record.unit === 'W.h' && /^-?\d+(\.\d+)?$/.test(value)) {
        const places = asset === 'multical-601' ? 6 : 3;
        const [whole, fraction = ''] = value.replace('-', '').split('.');
        const digits = whole.padStart(places + 1, '0');
        const decimals = (digits.slice(-places) + fraction).replace(/0+$/, '');
        const integral = digits.slice(0, -places).replace(/^0+(?=\d)/, '');
        return { value: (value.startsWith('-') ? '-' : '') + integral + (decimals ? '.' + decimals : ''), unit: places === 6 ? 'MWh' : 'kWh' };
    }
    const units = { Cel: '°C', m3: 'm³', 'm3.h-1': 'm³/h', 'W.h': 'Wh' };
    return { value, unit: units[record.unit] || record.unit || '' };
}

export function renderProduct(container, decoded, reportError) {
    const meter = decoded.meter?.identity;
    const view = element('section', 'product-view');
    container.replaceChildren(view);
    if (!meter && !decoded.records.length) {
        view.append(element('p', '', 'This telegram contains no meter identity or readings.'));
        return;
    }
    const fixture = identifyModel(meter, decoded.records);
    const family = deviceFamily(meter?.device_type_code);
    const asset = fixture?.asset || 'generic-' + family;
    const header = element('header', 'product-header');
    const name = fixture?.name || [meter?.manufacturer_code, meter?.device_type].filter(Boolean).join(' · ') || 'Unidentified M-Bus device';
    header.append(element('h2', '', name));
    const website = decoded.enrichment?.manufacturer_website;
    const reference = fixture?.source || (website && /^[a-z0-9.-]+$/i.test(website) ? 'https://' + website : null);
    if (reference) {
        const source = element('a', 'product-source', 'Manufacturer reference');
        source.href = reference;
        source.target = '_blank';
        source.rel = 'noopener noreferrer';
        header.append(source);
    }

    const stage = document.createElement('meter-turntable');
    stage.configure(asset, name, meter, decoded.records);
    const preview = element('div', 'product-preview');
    const hint = element('div', 'product-preview-footer', 'Drag or use arrow keys to rotate');
    const report = element('button', 'product-report', 'Report an error');
    report.type = 'button';
    report.addEventListener('click', reportError);
    preview.append(stage, report, hint);

    const details = element('div', 'product-readings');
    const measurement = element('div', 'product-measurement');
    const label = element('span', '');
    const value = element('strong', '');
    measurement.append(label, value);
    details.append(measurement);
    const records = decoded.records.filter(({value}) => typeof value.value === 'number'
        || (typeof value.value === 'string' && value.value.trim() !== ''));
    function showReading(record) {
        const reading = displayReading(record, asset);
        if (asset === 'gmc-emmod206' && record?.subunit && ['V', 'A'].includes(reading.unit)) {
            reading.phases = [1, 2, 3].map(phase => records.find(r => r.subunit === phase
                && r.unit === record.unit && r.storage_number === record.storage_number
                && r.tariff === record.tariff && r.function === record.function)?.value.value);
        }
        stage.updateReading(reading);
        label.textContent = record ? readingLabel(record) : 'Reading';
        value.textContent = record ? [reading.value, reading.unit].filter(Boolean).join(' ') : 'No decoded measurement';
    }
    const initial = primaryRecord(records, family);
    showReading(initial);
    if (records.length > 1) {
        const select = element('select', 'product-record-select');
        select.setAttribute('aria-label', 'Decoded reading');
        for (const r of records) select.add(new Option('#' + r.index + ' · ' + readingLabel(r), String(r.index), false, r === initial));
        select.addEventListener('change', () => showReading(records[select.selectedIndex]));
        measurement.append(select);
    }
    for (const [title, data] of [['Meter ID', meter?.id], ['Meter type', meter?.device_type]]) {
        if (data == null) continue;
        const item = element('div', '');
        item.append(element('span', '', title), element('strong', '', data));
        details.append(item);
    }
    const generic = asset.startsWith('generic-') || asset === 'wep-room-sensor';
    const caption = asset === 'elster-gas' ? 'BK-G4 reference housing · Exact model unknown'
        : asset === 'landisplusgyr-g350' ? 'E6 single-pipe reference housing'
        : asset === 'sensus-iperl' ? 'iPERL reference housing · DN15 illustration'
        : fixture && !fixture.asset ? 'Generic ' + family + ' meter illustration'
        : generic ? 'Device-type illustration' : 'Approximate 3D housing';
    view.append(header, preview, stage.controls, element('p', 'product-caption', caption), details);
}
