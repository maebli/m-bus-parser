export const caseWhite = '#dedfd8';
const grey = '#aeb5b4';
const slate = '#46545b';
const blue = '#405e90';
const red = '#b43736';

function rail(size, terminals, screen, keys, options = {}) {
    return { mount: 'rail', size, terminals, screen, keys, ...options };
}

function calculator(size, screen, keys, glands, options = {}) {
    return { mount: 'wall', size, screen, keys, glands, ...options };
}

function inline(size, span, screen, keys, options = {}) {
    return { mount: 'pipe', size, span, screen, keys, ...options };
}

const cfScreen = [0.29, 0.12, 0.46, 0.26];
const cfKeys = [
    [0.86, 0.25, red, '◀'],
    [0.83, 0.71, slate]
];
const cfCase = { rim: grey, panel: [0.08, 0.08, 0.84, 0.35, '#a7adad'] };
const deltaScreen = [0.1, 0.19, 0.51, 0.45];
const deltaKeys = [
    [0.75, 0.4, grey],
    [0.88, 0.4, grey]
];

export const profiles = {
    'multical-601': calculator(
        [147, 100, 52],
        [0.19, 0.17, 0.37, 0.27],
        [
            [0.096, 0.235, '#307c80', '▶'],
            [0.096, 0.41, grey, '▤']
        ],
        0,
        { ribbedRight: true, ears: true }
    ),
    'abb-delta': rail([122.5, 97, 64.8], 7, deltaScreen, deltaKeys),
    'berg-dzplus': rail([122.5, 97, 64.8], 7, deltaScreen, deltaKeys),
    'eastron-sdm630': rail(
        [72, 100, 66],
        4,
        [0.09, 0.1, 0.64, 0.8],
        [0.18, 0.39, 0.61, 0.82].map((y) => [0.86, y, grey]),
        { face: slate, case: caseWhite }
    ),
    'emh-diz': rail([107, 90, 62], 8, [0.12, 0.13, 0.58, 0.52], [[0.82, 0.41, slate]]),
    'emu-professional-375': rail(
        [90, 90, 70],
        4,
        [0.06, 0.1, 0.6, 0.78],
        [
            [0.78, 0.3, grey],
            [0.78, 0.7, grey]
        ],
        { caps: [red, blue], case: caseWhite, face: '#aec1cd' }
    ),
    'finder-7e-23-8-230-0020': rail([17.5, 89.2, 58], 1, [0.08, 0.12, 0.84, 0.42], [[0.5, 0.78, grey]]),
    'nzr-dhz-5-63': rail([107.5, 89.5, 64], 8, [0.08, 0.22, 0.5, 0.55], [[0.74, 0.48, grey]]),
    'saia-burgess-ale3': rail(
        [70, 90, 62],
        4,
        [0.2, 0.08, 0.6, 0.44],
        [
            [0.37, 0.76, slate, '▲'],
            [0.64, 0.76, slate, '▼']
        ],
        { caps: [blue, blue], backlight: true }
    ),
    'allmess-cf50': calculator(
        [120, 148, 54],
        [0.23, 0.15, 0.54, 0.18],
        [
            [0.17, 0.44, grey, '◀'],
            [0.83, 0.44, grey, '▶'],
            [0.5, 0.72, slate]
        ],
        4,
        { terminalCover: true }
    ),
    'itron-cf-51': calculator([149, 119, 55], cfScreen, cfKeys, 4, cfCase),
    'itron-cf-55': calculator([149, 119, 55], cfScreen, cfKeys, 5, cfCase),
    'itron-cf-echo-ii': inline([126, 120, 54], 260, cfScreen, cfKeys, { ...cfCase, flange: true }),
    'elster-f2': calculator([90, 112, 32], [0.2, 0.15, 0.6, 0.18], [[0.39, 0.47, slate]], 2),
    'svm-f22': calculator([102, 140, 48], [0.2, 0.16, 0.6, 0.16], [[0.38, 0.48, slate]], 3, { ears: true }),
    'engelmann-sensostar-2c': calculator(
        [123.7, 198, 45.8],
        [0.08, 0.24, 0.46, 0.13],
        [
            [0.18, 0.45, slate],
            [0.42, 0.48, slate]
        ],
        5,
        { probes: true }
    ),
    'minol-minocal-wr3': calculator(
        [135, 115, 48],
        [0.24, 0.15, 0.54, 0.24],
        [
            [0.64, 0.57, grey],
            [0.8, 0.57, slate]
        ],
        4,
        { rim: slate }
    ),
    'sontex-supercal-531': calculator(
        [138.7, 110, 45.3],
        [0.09, 0.16, 0.52, 0.27],
        [
            [0.46, 0.56, grey, '▼'],
            [0.61, 0.56, grey, '▶']
        ],
        6
    ),
    'sensus-pollutherm': calculator(
        [120, 150, 53],
        [0.1, 0.29, 0.49, 0.16],
        [
            [0.33, 0.54, red],
            [0.69, 0.61, slate]
        ],
        4
    ),
    'kamstrup-382-005': calculator(
        [167, 202, 65],
        [0.16, 0.19, 0.55, 0.14],
        [
            [0.24, 0.41, grey],
            [0.47, 0.41, slate],
            [0.7, 0.41, grey]
        ],
        0,
        { terminalCover: true }
    ),
    'engelmann-sensostar-2': inline(
        [76, 82, 30],
        110,
        [0.17, 0.24, 0.65, 0.21],
        [
            [0.36, 0.65, slate],
            [0.67, 0.66, grey]
        ],
        { radius: 0.19 }
    ),
    'engelmann-waterstar': inline([72, 72, 29], 110, [0.19, 0.3, 0.62, 0.24], [], {
        round: true,
        rim: slate
    }),
    'itron-integral-mk-maxx': inline([74, 74, 29], 110, [0.18, 0.26, 0.63, 0.25], [[0.64, 0.64, red]], {
        round: true,
        rim: red
    }),
    'landisplusgyr-ultraheat-t230': inline(
        [69, 105, 36],
        110,
        [0.16, 0.38, 0.64, 0.19],
        [
            [0.35, 0.72, grey],
            [0.73, 0.72, slate]
        ],
        { composite: true }
    ),
    'metrona-ultraheat-xs': inline([88, 112, 40], 130, [0.09, 0.12, 0.59, 0.17], [[0.76, 0.72, slate]], {
        rim: '#586e79'
    }),
    'minol-minocal-c2': inline([75, 86, 36], 110, [0.19, 0.69, 0.6, 0.15], [[0.72, 0.41, '#dec733']], {
        face: red,
        rim: red
    }),
    'sensus-pollucom-e': inline([78, 77, 36], 110, [0.16, 0.22, 0.62, 0.23], [[0.55, 0.59, red]], {
        rim: blue,
        probe: true,
        radius: 0.18
    }),
    'sensus-pollustat': inline(
        [103, 79, 37],
        130,
        [0.17, 0.15, 0.64, 0.24],
        [
            [0.32, 0.68, slate],
            [0.7, 0.68, slate]
        ],
        { panel: [0.09, 0.07, 0.82, 0.39, slate] }
    ),
    'sensus-pollustat-e': inline(
        [84, 96, 36],
        130,
        [0.13, 0.19, 0.57, 0.19],
        [
            [0.34, 0.55, red],
            [0.7, 0.59, slate]
        ],
        { probe: true }
    ),
    'siemens-wfh21': inline([69, 86, 35], 110, [0.13, 0.21, 0.67, 0.2], [[0.52, 0.65, slate]], {
        upright: true,
        rim: slate
    }),
    'elster-falcon': { mount: 'pickup', size: [76, 30, 18], radius: 0.44, cord: -1, face: '#bcc8c5' },
    'itron-cyble-m-bus-v1-4': { mount: 'pickup', size: [68, 54, 24], radius: 0.35, cord: 1, face: '#d7d4c5' },
    'relay-padpuls-2': { mount: 'module', size: [80, 80, 52], ears: true, knockouts: true },
    'wep-room-sensor': { mount: 'sensor', size: [82, 82, 25] },
    'generic-electricity': rail([72, 90, 60], 4, [0.1, 0.18, 0.64, 0.6], [[0.85, 0.48, grey]]),
    'generic-heat': calculator([125, 97, 48], [0.15, 0.17, 0.64, 0.27], [[0.72, 0.63, grey]], 3),
    'generic-sensor': { mount: 'sensor', size: [72, 84, 26] },
    'generic-device': { mount: 'module', size: [92, 67, 32], ears: true }
};
