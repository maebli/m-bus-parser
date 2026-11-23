const fixtures = [
    ['ABB/78563412/2/2', 'ABB DELTA', 'abb-delta',
        'https://library.e.abb.com/public/938bcd468d07d56fc1257084004c5539/2CDC512062D0201.pdf'],
    ['SLB/02205100/2/4', 'Allmess CF50', 'allmess-cf50',
        'https://www.entreprises.gouv.fr/files/files/Entites/Metrologie/decisions-approbation/1996/96005820091.pdf'],
    ['ABB/00000000/2/2', 'Berg DZ+', 'berg-dzplus',
        'https://library.e.abb.com/public/dad4a3400e8bd755c12578300054ddf7/DZplus%20und%20DELTAplus_Bed_Anl.pdf'],
    ['PAD/21346578/1/2', 'Eastron SDM630', 'eastron-sdm630',
        'https://www.eastrongroup.com/eastrongroup/2024/08/21/eastron%C2%A0sdm630-mbus%C2%A0v2%C2%A0user%C2%A0manual%C2%A02020%C2%A0v1.3.pdf'],
    ['SVM/00802657/8/4', 'Elster F2', 'elster-f2',
        'https://kamstrup-delivery.sitecorecontenthub.cloud/api/public/content/60336-downloadOriginal?v=11080f8d'],
    ['ELS/70112345/10/7', 'Elster Falcon', 'elster-falcon',
        'https://prod-edam.honeywell.com/content/dam/honeywell-edam/pmt/hps/products/sme/water/digital-metering/probes---pulsers/pr6-pr7/FalconMB-M-1404e-0511.pdf?download=false'],
    ['EMH/00623702/0/2', 'EMH DIZ', 'emh-diz',
        'https://emh-metering.com/wp-content/uploads/2025/01/DIZ-G-EMH-BIA-E-1.61-compressed.pdf'],
    ['EMU/00032629/16/2', 'EMU Professional 375', 'emu-professional-375',
        'https://www.emuag.ch/files/datasheet/Datasheet_Professional_EN.pdf'],
    ['EFE/24083345/0/4', 'Engelmann SensoStar 2', 'engelmann-sensostar-2',
        'https://www.engelmann.de/en/product/sensostar/'],
    ['EFE/10380010/1/4', 'Engelmann SensoStar 2C', 'engelmann-sensostar-2c',
        'https://d3pcsg2wjq9izr.cloudfront.net/files/97600/download/746130/1-2.pdf'],
    ['EFE/04990254/0/6', 'Engelmann WaterStar', 'engelmann-waterstar',
        'https://www.engelmann.de/en/product/waterstar-m/'],
    ['FIN/23006207/35/2', 'Finder 7E.23.8.230.0020', 'finder-7e-23-8-230-0020',
        'https://cdn.findernet.com/app/uploads/2020/09/11040742/IB7E23-2.pdf'],
    ['GMC/12345678/230/2', 'GMC SINEAX A230 + EMMOD206', 'gmc-emmod206',
        'https://www.gossenmetrawattusa.com/media/88698/sineax-a230-db_gb.pdf'],
    ['GWF/00182007/53/7', 'GWF MTKcoder', 'gwf-mtkcoder',
        'https://productfinder.gwf.ch/en/export?id=4062'],
    ['ACW/11490378/14/22', 'Itron BM +m', 'itron-bm-plusm',
        'https://emea.itron.com/o/commerce-media/accounts/-1/attachments/3829681'],
    ['ACW/11155185/10/13', 'Itron CF 51', 'itron-cf-51',
        'https://emea.itron.com/products/cf-51'],
    ['ACW/11127667/11/12', 'Itron CF 55', 'itron-cf-55',
        'https://aunz.itron.com/products/cf-55'],
    ['ACW/11100091/9/4', 'Itron CF Echo II', 'itron-cf-echo-ii',
        'https://na.itron.com/documents/44647/3815900/CF%2BEcho%2BII%2BEN_1122.pdf/602b1921-fa88-109d-36d9-1884edf8529e?t=1711249129441&version=1.1'],
    ['ACW/12000071/20/7', 'Itron Cyble M-Bus v1.4', 'itron-cyble-m-bus-v1-4',
        'https://na.itron.com/documents/44647/3809491/CYBLE%2BM-BUS%2Big%2BML%2Bweb.pdf/6abe1e9e-a36d-1cec-8a09-8f7634bf5636'],
    ['ACW/10020387/20/3', 'Itron Cyble M-Bus v1.4', 'itron-cyble-m-bus-v1-4',
        'https://na.itron.com/documents/44647/3809491/CYBLE%2BM-BUS%2Big%2BML%2Bweb.pdf/6abe1e9e-a36d-1cec-8a09-8f7634bf5636'],
    ['SLB/11817314/6/4', 'Itron Integral MK MaXX', 'itron-integral-mk-maxx',
        'https://actaris.nt-rt.ru/images/manuals/Integral-MK%20MaXX.pdf'],
    ['KAM/14839120/1/2', 'Kamstrup 382-005', 'kamstrup-382-005',
        'https://www.kamstrup.com/en-en/product-centre/kamstrup-382'],
    ['KAM/06855817/8/4', 'Kamstrup MULTICAL 601', 'multical-601',
        'https://kamstrup-delivery.sitecorecontenthub.cloud/api/public/content/60944-downloadOriginal?v=6d36e7ae'],
    ['LGB/12082058/64/3', 'Landis+Gyr G350', 'landisplusgyr-g350',
        'https://oms-group.org/wp-content/uploads/2024/08/171001_Gaszaehler_Datenblatt_GermanG4_1und2rohr.pdf#page=2'],
    ['LUG/66660205/7/4', 'Landis+Gyr ULTRAHEAT T230', 'landisplusgyr-ultraheat-t230',
        'https://www.landisgyr.com/content/dam/landisgyr/products/apac-product-sheets/T230_Technical_Description_EN_g.pdf.coredownload.pdf'],
    ['LUG/01810054/2/4', 'Metrona ULTRAHEAT XS', 'metrona-ultraheat-xs',
        'https://www.brunata-metrona.de/downloads/allgemein/hh/BRUNATA_Waermezaehler_Ultraheat_XS_Technisches_Datenblatt.pdf'],
    ['ZRM/31425084/129/4', 'Minol Minocal C2', 'minol-minocal-c2',
        'https://www.minol.de/wp-content/uploads/sites/9/2025/08/Minol-Montageanleitung-Waermezaehler-Minocal-Combi.pdf'],
    ['ZRM/31802759/130/4', 'Minol Minocal WR3', 'minol-minocal-wr3',
        'https://www.minol.de/wp-content/uploads/sites/9/2025/08/Minol-Datenblatt-Minocal-WR-3-Rechenwerk-fuer-Waerme-und-Kaeltezaehler.pdf'],
    ['NZR/30100608/1/2', 'NZR DHZ 5/63', 'nzr-dhz-5-63',
        'https://www.nzr.de/fileadmin/user_upload/pdf-dokumente/Datenbl%C3%A4tter/Elektrizit%C3%A4t/DHZ__Datenblatt.pdf'],
    ['REL/11216301/65/3', 'Relay PadPuls 2', 'relay-padpuls-2',
        'https://www.relay.de/en/products/m-bus-slaves/padpuls-m2'],
    ['SBC/19000055/22/2', 'Saia-Burgess ALE3', 'saia-burgess-ale3',
        'https://prod-edam.honeywell.com/content/dam/honeywell-edam/hbt/en-us/documents/literature-and-specs/datasheets/sbc/26-520_ENG_DS_EnergyMeter-ALE3-with-M-Bus.pdf'],
    ['SEN/63940045/8/4', 'Sensus PolluCom E', 'sensus-pollucom-e',
        'https://www.xylem.com/en-ye/products--services/metrology-equipment-for-utilities/meters/pollucom-e-thermal-energy-meter/'],
    ['SEN/00011788/6/13', 'Sensus PolluStat', 'sensus-pollustat',
        'https://www.sensus-international.com/en/products/pollustat'],
    ['SEN/21265095/14/4', 'Sensus PolluStat E', 'sensus-pollustat-e',
        'https://www.heatingandprocess.com/wp-content/uploads/2016/05/Sensus-PolluStat-E-Heat-Meter-Product-Brochure.pdf'],
    ['SPX/21050076/49/4', 'Sensus PolluTherm', 'sensus-pollutherm',
        'https://www.sensus-international.com/en/products/pollutherm'],
    ['LSE/08006491/153/6', 'Siemens WFH21', 'siemens-wfh21',
        'https://sid.siemens.com/api/khub/documents/kbGyZOf2y3kQaTvVbIS_yg/content'],
    ['SON/08420624/13/4', 'Sontex Supercal 531', 'sontex-supercal-531',
        'https://sontex.ch/wp-content/uploads/2019/02/supercal531-mid-ig-de-en-1.pdf'],
    ['SVM/01006089/9/12', 'SVM F22', 'svm-f22',
        'https://kamstrup-delivery.sitecorecontenthub.cloud/api/public/content/59828-downloadOriginal?v=542d44ac'],
    ['WEP/00013482/2/27', 'WEP room sensor', 'wep-room-sensor',
        'https://www.weptech.de/en/products/sensors.html'],
    ['ELS/12345678/51/3', 'Elster gas meter', 'elster-gas', 'https://meterbuy.com/wp-content/uploads/160226-Elster-DS_BK_4_EN.pdf'],
];

const iperl = { name: 'Sensus iPERL', asset: 'sensus-iperl', source: 'https://www.sensus-international.com/en/products/iperl' };
const models = new Map(fixtures.map(([identity, name, asset, source]) => [identity, { name, asset, source }]));

const gwfWaterModels = [
    { name: 'UNICOcoder MP', media: [0x06, 0x07], source: 'https://productfinder.gwf.ch/en/unicocoder-mp' },
    { name: 'MTKcoder MP', media: [0x07], source: 'https://productfinder.gwf.ch/en/mtkcoder-mp' },
    { name: 'MTWcoder MP', media: [0x06], source: 'https://productfinder.gwf.ch/en/mtwcoder-mp' },
];

const multicalModels = [
    { model: '302', version: 0x30, media: [0x04, 0x0c, 0x0d] },
    { model: '303', version: 0x40, media: [0x04, 0x0c, 0x0d] },
    { model: '402', version: 0x19, media: [0x04] },
    { model: '403', version: 0x34, media: [0x04, 0x0a, 0x0b, 0x0c, 0x0d] },
    { model: '602', version: 0x1c, media: [0x04] },
    { model: '603', version: 0x35, media: [0x04, 0x0c, 0x0d] },
    { model: '803', version: 0x39, media: [0x04] },
];

function normalizeModel(value) {
    return value.toLowerCase().replace(/[^a-z0-9]/g, '');
}

export function identifyModel(meter, records) {
    if (!meter) return;
    const reported = records.find(record => record.value.kind === 'text'
        && record.quantities.some(quantity => /^model(?: or |\s*\/\s*)version$/i.test(quantity)));
    if (typeof reported?.value.value === 'string') {
        const name = normalizeModel(reported.value.value);
        if (meter.manufacturer_code === 'GWF') {
            const match = gwfWaterModels.find(model => model.media.includes(meter.device_type_code)
                && normalizeModel(model.name) === name.replace(/^gwf/, ''));
            if (match) return { name: 'GWF ' + match.name, source: match.source };
        }
        for (const [key, model] of models) {
            if (!key.startsWith(meter.manufacturer_code + '/') || !key.endsWith('/' + meter.device_type_code)) continue;
            const shortName = model.name.slice(model.name.indexOf(' ') + 1);
            if (name === normalizeModel(model.name) || name === normalizeModel(shortName)) return model;
        }
    }
    const fixture = models.get([meter.manufacturer_code, meter.id, meter.version, meter.device_type_code].join('/'));
    if (fixture) return fixture;
    if (meter.manufacturer_code === 'GWF' && meter.version === 0x3c && [0x06, 0x07].includes(meter.device_type_code)) {
        return { name: 'GWFcoder MP water meter' };
    }
    if (meter.manufacturer_code === 'KAM') {
        const match = multicalModels.find(model => model.version === meter.version && model.media.includes(meter.device_type_code));
        if (match) return { name: 'Kamstrup MULTICAL ' + match.model };
    }
    if (meter.manufacturer_code === 'SEN') {
        if (meter.device_type_code === 0x07 && [0x0b, 0x68, 0x7c].includes(meter.version)) return iperl;
        if (meter.device_type_code === 0x06 && meter.version === 0x68) return iperl;
    }
}
