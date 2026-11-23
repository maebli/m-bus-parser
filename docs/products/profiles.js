export const caseWhite = '#dedfd8';
const grey = '#aeb5b4';
const slate = '#46545b';
const blue = '#405e90';
const red = '#b43736';

function rail(size, terminals, screen, keys, options = {}) {
    return { mount: 'rail', size, terminals, screen, keys, fascia: 0.55, ...options };
}

function calculator(size, screen, keys, options = {}) {
    return { mount: 'wall', size, screen, keys, ...options };
}

function inline(size, pipe, screen, keys, options = {}) {
    return { mount: 'pipe', size, pipe, screen, keys, ...options };
}

export const profiles = {
    'multical-601': calculator(
        [1.8, 1.224, 0.637],
        [0.17, 0.17, 0.35, 0.26],
        [
            [0.096, 0.235, '#307c80', '▶'],
            [0.096, 0.41, grey, '▤']
        ],
        { ribbedRight: true, radius: 0.035, mountingPlate: 0.047, cameraRadius: 1.144 }
    ),
    'abb-delta': rail(
        [1.8, 1.516, 0.895],
        7,
        [0.082, 0.407, 0.358, 0.353],
        [
            [0.51, 0.48, '#84908b'],
            [0.51, 0.72, '#84908b']
        ],
        { covered: true, strip: '#b34a43', cameraRadius: 1.172 }
    ),
    'berg-dzplus': rail(
        [1.458, 1.72, 1.015],
        7,
        [0.094, 0.4, 0.68, 0.345],
        [
            [0.87, 0.38, '#84908b'],
            [0.87, 0.72, '#84908b']
        ],
        { cameraRadius: 1.124 }
    ),
    'eastron-sdm630': rail(
        [1.238, 1.72, 1.135],
        4,
        [0.14, 0.192, 0.59, 0.541],
        [
            [0.86, 0.18, grey],
            [0.86, 0.39, grey],
            [0.86, 0.61, grey],
            [0.86, 0.82, grey]
        ],
        { face: slate, case: caseWhite, fascia: 0.45, covered: true, cameraRadius: 1.135 }
    ),
    'emh-diz': rail([1.37, 1.72, 1.015], 8, [0.14, 0.301, 0.619, 0.373], [[0.82, 0.41, slate]], {
        cameraRadius: 1.119
    }),
    'emu-professional-375': rail(
        [1.695, 1.72, 1.015],
        4,
        [0.086, 0.294, 0.617, 0.501],
        [
            [0.78, 0.3, grey],
            [0.78, 0.7, grey]
        ],
        { caps: [red, blue], case: caseWhite, face: '#aec1cd', covered: true, cameraRadius: 1.259 }
    ),
    'finder-7e-23-8-230-0020': rail(
        [0.337, 1.72, 1.118],
        1,
        [0.236, 0.237, 0.447, 0.484],
        [[0.38, 0.91, slate]],
        { fascia: 0.529, shoulder: 0.733, terminalDepth: 0.414, shoulderDepth: 0.759, cameraRadius: 0.975 }
    ),
    'nzr-dhz-5-63': rail([1.8, 1.415, 0.835], 8, [0.091, 0.27, 0.487, 0.397], [[0.74, 0.48, grey]], {
        covered: true,
        cameraRadius: 1.147
    }),
    'saia-burgess-ale3': rail(
        [1.338, 1.72, 1.015],
        4,
        [0.147, 0.355, 0.625, 0.378],
        [
            [0.85, 0.35, slate, '▲'],
            [0.85, 0.72, slate, '▼']
        ],
        { caps: [blue, blue], backlight: true, cameraRadius: 1.096 }
    ),
    'allmess-cf50': calculator(
        [1.52, 1.426, 0.684],
        [0.16, 0.258, 0.534, 0.232],
        [
            [0.87, 0.27, slate],
            [0.87, 0.46, slate]
        ],
        {
            terminalCover: true,
            glands: [-0.489, -0.167, 0.165, 0.485],
            glandRadius: 0.086,
            glandLength: 0.248,
            wingSpan: 1.801,
            cameraRadius: 1.097
        }
    ),
    'itron-cf-51': calculator(
        [1.572, 1.234, 0.684],
        [0.23, 0.256, 0.468, 0.216],
        [
            [0.85, 0.26, red],
            [0.85, 0.42, slate]
        ],
        {
            rim: grey,
            panel: [0.04, 0.09, 0.91, 0.53, '#c5c8c3'],
            glands: [-0.474, -0.161, 0.141, 0.447],
            glandRadius: 0.086,
            glandLength: 0.239,
            wingSpan: 1.801,
            cameraRadius: 1.084
        }
    ),
    'itron-cf-55': calculator(
        [1.595, 1.166, 0.684],
        [0.28, 0.292, 0.441, 0.184],
        [
            [0.44, 0.69, slate],
            [0.58, 0.69, slate]
        ],
        {
            rim: grey,
            panel: [0.16, 0.25, 0.65, 0.35, '#596368'],
            glands: [-0.527, -0.267, 0.004, 0.275, 0.529],
            glandRadius: 0.086,
            glandLength: 0.19,
            wingSpan: 1.801,
            cameraRadius: 1.046
        }
    ),
    'itron-cf-echo-ii': inline(
        [0.95, 0.804, 0.265],
        [-1.59, -0.038, 0.137, 0.137],
        [0.364, 0.13, 0.396, 0.221],
        [
            [0.86, 0.25, red, '◀'],
            [0.83, 0.71, slate]
        ],
        { rim: grey, panel: [0.08, 0.08, 0.84, 0.35, '#a7adad'], flange: true, cameraRadius: 1.267 }
    ),
    'elster-f2': calculator([1.478, 1.72, 0.562], [0.198, 0.21, 0.601, 0.142], [[0.39, 0.47, slate]], {
        cameraRadius: 1.158
    }),
    'svm-f22': calculator([1.266, 1.57, 0.481], [0.247, 0.257, 0.503, 0.136], [[0.38, 0.46, slate]], {
        topLug: 0.157,
        cameraRadius: 1.04
    }),
    'engelmann-sensostar-2c': calculator(
        [1.106, 1.72, 0.42],
        [0.09, 0.239, 0.441, 0.093],
        [
            [0.18, 0.45, slate],
            [0.42, 0.48, slate]
        ],
        { probes: true, cameraRadius: 1.322 }
    ),
    'minol-minocal-wr3': calculator(
        [1.772, 1.492, 0.676],
        [0.27, 0.279, 0.461, 0.166],
        [
            [0.66, 0.8, grey],
            [0.84, 0.8, slate]
        ],
        {
            rim: slate,
            glands: [-0.466, -0.156, 0.161, 0.476],
            glandRadius: 0.085,
            glandLength: 0.219,
            wingSpan: 1.788,
            radius: 0.21,
            sideGrips: true,
            cameraRadius: 1.107
        }
    ),
    'sontex-supercal-531': calculator(
        [1.523, 1.371, 0.684],
        [0.192, 0.261, 0.511, 0.238],
        [
            [0.85, 0.24, slate, '▲'],
            [0.85, 0.39, slate, '▼']
        ],
        {
            glands: [-0.452, -0.157, 0.149, 0.459],
            glandRadius: 0.086,
            glandLength: 0.265,
            wingSpan: 1.801,
            panel: [0.05, 0.12, 0.91, 0.4, '#a6adac'],
            cameraRadius: 1.095
        }
    ),
    'sensus-pollutherm': calculator(
        [1.296, 1.72, 0.493],
        [0.085, 0.268, 0.457, 0.149],
        [
            [0.33, 0.54, red],
            [0.69, 0.61, slate]
        ],
        { cameraRadius: 1.106 }
    ),
    'kamstrup-382-005': calculator(
        [1.173, 1.72, 0.446],
        [0.277, 0.218, 0.423, 0.111],
        [
            [0.24, 0.41, grey],
            [0.47, 0.41, slate],
            [0.7, 0.41, grey]
        ],
        { terminalCover: true, cameraRadius: 1.026 }
    ),
    'engelmann-sensostar-2': inline(
        [1.384, 1.455, 0.457],
        [-1.087, 1.113, -0.061, 0.212],
        [0.19, 0.3, 0.63, 0.219],
        [
            [0.36, 0.65, slate],
            [0.67, 0.66, grey]
        ],
        { radius: 0.29, cameraRadius: 1.191 }
    ),
    'engelmann-waterstar': inline(
        [1.13, 1.161, 0.373],
        [-1.099, 1.101, -0.036, 0.178],
        [0.18, 0.341, 0.635, 0.217],
        [],
        { round: true, rim: slate, cameraRadius: 1.162 }
    ),
    'itron-integral-mk-maxx': inline(
        [1.366, 1.441, 0.451],
        [-1.091, 1.109, -0.135, 0.217],
        [0.189, 0.316, 0.635, 0.223],
        [[0.75, 0.68, slate]],
        { rim: '#a6aaa4', radius: 0.31, cameraRadius: 1.202 }
    ),
    'landisplusgyr-ultraheat-t230': inline(
        [1.197, 1.868, 0.395],
        [-1.186, -0.06, 0.02, 0.197],
        [0.274, 0.442, 0.527, 0.12],
        [
            [0.35, 0.72, grey],
            [0.73, 0.72, slate]
        ],
        { composite: true, cameraRadius: 1.467 }
    ),
    'metrona-ultraheat-xs': inline(
        [1.475, 2.191, 0.487],
        [-0.988, 0.979, 0.349, 0.212],
        [0.116, 0.198, 0.559, 0.082],
        [[0.76, 0.72, slate]],
        { rim: '#586e79', taper: 0.22, cameraRadius: 1.364 }
    ),
    'minol-minocal-c2': inline(
        [1.441, 2.059, 0.476],
        [-1.095, 1.105, 0.163, 0.23],
        [0.14, 0.713, 0.721, 0.169],
        [
            [0.22, 0.51, slate],
            [0.75, 0.51, '#dec733']
        ],
        { face: red, rim: red, taper: 0.14, cameraRadius: 1.283 }
    ),
    'sensus-pollucom-e': inline(
        [1.443, 1.652, 0.476],
        [-1.112, 1.088, -0.234, 0.204],
        [0.179, 0.268, 0.622, 0.197],
        [[0.55, 0.59, red]],
        { rim: blue, radius: 0.29, cameraRadius: 1.225 }
    ),
    'sensus-pollustat': inline(
        [1.953, 1.346, 0.444],
        [-1.218, 0.982, -0.037, 0.228],
        [0.209, 0.173, 0.548, 0.212],
        [
            [0.32, 0.68, slate],
            [0.7, 0.68, slate]
        ],
        { panel: [0.09, 0.07, 0.82, 0.39, slate], cameraRadius: 1.411 }
    ),
    'sensus-pollustat-e': inline(
        [1.298, 1.621, 0.428],
        [-0.941, 0.939, 0.076, 0.171],
        [0.111, 0.303, 0.397, 0.142],
        [
            [0.34, 0.55, red],
            [0.7, 0.59, slate]
        ],
        { coil: true, cameraRadius: 1.361 }
    ),
    'siemens-wfh21': inline(
        [1.338, 1.814, 0.442],
        [-0.933, 0.948, -1.262, 0.164],
        [0.156, 0.286, 0.694, 0.125],
        [[0.52, 0.65, slate]],
        { upright: true, rim: slate, cameraRadius: 1.463 }
    ),
    'elster-falcon': {
        mount: 'pickup',
        size: [1.453, 0.755, 0.684],
        radius: 0.44,
        face: '#bcc8c5',
        cord: [0.04, -1.073, 0.046],
        ribs: true,
        cameraRadius: 1.015
    },
    'itron-cyble-m-bus-v1-4': {
        mount: 'pickup',
        size: [1.616, 0.967, 0.684],
        radius: 0.35,
        face: '#d7d4c5',
        cord: [-0.246, 0.992, -0.393],
        notch: true,
        cameraRadius: 0.993
    },
    'relay-padpuls-2': { mount: 'module', size: [1.526, 1.72, 0.58], knockouts: true, cameraRadius: 1.156 },
    'wep-room-sensor': { mount: 'sensor', size: [1.692, 1.72, 0.643], radius: 0.2, cameraRadius: 1.139 },
    'generic-electricity': rail([1.423, 1.72, 1.015], 4, [0.091, 0.314, 0.636, 0.347], [[0.85, 0.48, grey]], {
        cameraRadius: 1.141
    }),
    'generic-heat': calculator([1.51, 1.39, 0.684], [0.23, 0.22, 0.54, 0.22], [[0.71, 0.63, grey]], {
        glands: [-0.38, 0, 0.38],
        glandRadius: 0.09,
        glandLength: 0.25,
        wingSpan: 1.816,
        radius: 0.17,
        cameraRadius: 1.079
    }),
    'generic-sensor': { mount: 'sensor', size: [1.681, 1.72, 0.639], radius: 0.22, cameraRadius: 1.089 },
    'generic-device': {
        mount: 'module',
        size: [1.711, 1.32, 0.65],
        footTerminal: true,
        glands: [0, 0.45],
        glandRadius: 0.087,
        glandLength: 0.27,
        topLug: 0.14,
        cameraRadius: 1.076
    }
};
