import { flowBody, pipeShell } from './pipe.js';
import { caseWhite } from './profiles.js';

const steel = [153, 158, 152];
const graphite = [62, 66, 64];
const brass = [191, 145, 72];
const fullFace = [0, 0, 1, 1];

function rgb(hex) {
    return hex.match(/[0-9a-f]{2}/gi).map((pair) => parseInt(pair, 16));
}

function railHousing(profile, { box, solid, circle }) {
    const [frontWidth, h, d] = profile.size;
    const w = frontWidth * 1.009;
    const plastic = rgb(profile.case || caseWhite);
    const shoulder = profile.shoulder || 0.73;
    const capDepth = d * (profile.terminalDepth || 0.56);
    const stepDepth = d * (profile.shoulderDepth || 0.79);
    for (const [index, side] of [1, -1].entries()) {
        const cap = profile.caps ? rgb(profile.caps[index]) : plastic;
        const capCenter = (side * h * (1 + shoulder)) / 4;
        box(w, (h * (1 - shoulder)) / 2, capDepth, 0, capCenter, (capDepth - d) / 2, cap, 0.012);
        box(
            w,
            (h * (shoulder - profile.fascia)) / 2,
            stepDepth,
            0,
            (side * h * (shoulder + profile.fascia)) / 4,
            (stepDepth - d) / 2,
            plastic,
            0.012
        );
        if (profile.covered) {
            box(w * 0.95, 0.024, 0.027, 0, capCenter, capDepth - d / 2, cap, 0.009);
        } else {
            const pitch = w / profile.terminals;
            const r = Math.min(h * 0.041, pitch * 0.28);
            for (let i = 0; i < profile.terminals; i++) {
                const x = -w / 2 + (i + 0.5) * pitch;
                solid(
                    circle,
                    [x, capCenter, capDepth - d / 2],
                    'z',
                    [
                        [0, r],
                        [0.003, r]
                    ],
                    graphite
                );
                box(r * 1.4, r * 0.25, 0.003, x, capCenter, capDepth - d / 2 + 0.004, steel, 0.001);
            }
        }
        box(w * 0.87, h * 0.2, 0.16, 0, side * h * 0.29, -d * 0.55, steel, 0.028);
        box(w * 0.81, 0.055, 0.13, 0, side * h * 0.19, -d * 0.61, graphite, 0.015);
    }
    box(w, h * profile.fascia, d, 0, 0, 0, plastic, 0.018, fullFace);
}

function flowHousing(profile, { face, solid, circle, cable, lead }) {
    const [w, h] = profile.size;
    const [left, right, y, r] = profile.pipe;
    const flanged = profile.flange;
    const chamber = flanged ? 1.13 : 1.55;
    const material = profile.composite ? graphite : brass;
    flowBody(face, left, right, y, r, material, flanged, chamber);
    const span = right - left;
    const saddleX = Math.max(left + span * 0.25, Math.min(0, right - span * (flanged ? 0.12 : 0.25)));
    if (profile.upright) {
        solid(
            circle,
            [saddleX, y, 0],
            'y',
            [
                [r * 0.92, r * 1.45],
                [-h / 2 - y + 0.012, r * 1.2]
            ],
            steel
        );
        return 0;
    }
    const rear = r * (flanged ? 3.25 : chamber) + 0.035;
    solid(
        circle,
        [saddleX, y, 0],
        'z',
        [
            [r * 0.92, r * 1.12],
            [rear - 0.018, r * 1.15],
            [rear + 0.012, r * 1.32]
        ],
        steel
    );
    if (flanged) {
        const pickupX = left + span * 0.52;
        solid(
            circle,
            [pickupX, y, 0],
            'z',
            [
                [r * 0.92, r * 1.45],
                [r * 1.38, r * 1.45]
            ],
            steel
        );
        lead(
            [pickupX, y - r * 0.66, r * 1.32],
            [pickupX - r * 0.46, y - r * 2.4, rear * 0.6],
            [-w * 0.47, y - r * 2.1, rear],
            [-w * 0.37, y - r * 1.3, rear + 0.01],
            0.021,
            graphite
        );
    }
    if (profile.composite) {
        const probeX = left + span * 0.295;
        solid(
            circle,
            [probeX, y, 0],
            'y',
            [
                [r * 0.9, r * 0.57],
                [r * 1.98, r * 0.57],
                [r * 2.1, r * 0.37]
            ],
            material
        );
        lead(
            [probeX, y + r * 2.07, 0],
            [probeX, h * 0.79, 0],
            [-w * 0.205, h * 0.82, rear],
            [-w * 0.118, h * 0.483, rear + 0.015],
            0.029,
            steel
        );
    }
    if (profile.coil) {
        const wire = [144, 48, 39];
        const coil = Array.from({ length: 121 }, (_, i) => {
            const t = i / 120;
            return [
                w * (-0.447 + 0.341 * Math.cos(5 * Math.PI * t)),
                h * (-0.579 - 0.233 * Math.sin(5 * Math.PI * t)),
                rear - 0.04 - 0.26 * t
            ];
        });
        lead(
            [-0.049 * w, -0.476 * h, rear + 0.12],
            [-0.03 * w, -0.514 * h, rear + 0.12],
            [-0.106 * w, -0.53 * h, rear - 0.04],
            coil[0],
            0.021,
            wire
        );
        cable(coil, 0.021, wire);
        const tip = [-0.744 * w, -0.152 * h, coil.at(-1)[2] - 0.035];
        lead(coil.at(-1), [-0.79 * w, -0.44 * h, tip[2]], [-0.77 * w, -0.277 * h, tip[2]], tip, 0.021, wire);
        solid(
            circle,
            tip,
            'y',
            [
                [0, 0.029],
                [0.11, 0.029],
                [0.14, 0.023]
            ],
            steel
        );
    }
    return rear + profile.size[2] / 2;
}

function temperatureProbes(w, h, { lead, solid, circle, face }) {
    for (const [gland, offset] of [
        [0.125, 0.26],
        [0.28, 0]
    ]) {
        const start = [(gland - 0.5) * w, -h * 0.48, 0];
        const tipX = w / 2 + 0.18 + offset;
        const bottom = -h / 2 - 0.32 - offset * 0.65;
        const bend = [(start[0] + tipX) / 2, bottom, 0];
        const probeY = -h / 2 - 0.04;
        lead(start, [start[0], bottom, 0], [start[0] + 0.12, bottom, 0], bend, 0.014, graphite);
        lead(bend, [tipX - 0.08, bottom, 0], [tipX, bottom, 0], [tipX, probeY, 0], 0.014, graphite);
        pipeShell(
            face,
            'y',
            [tipX, probeY, 0],
            [
                [-0.01, 0.027],
                [0.06, 0.032],
                [0.08, 0.046, true],
                [0.132, 0.046, true],
                [0.165, 0.035]
            ],
            0.017,
            brass
        );
        solid(
            circle,
            [tipX, probeY, 0],
            'y',
            [
                [0.14, 0.02],
                [0.33, 0.02],
                [0.337, 0.015]
            ],
            steel
        );
    }
}

export function buildHousing(profile, mesh) {
    if (profile.mount === 'rail') return railHousing(profile, mesh);
    const { face, solid, box, outline, circle, lead } = mesh;
    const [w, h, depth] = profile.size;
    const paint = rgb(profile.face || caseWhite);
    const rim = profile.rim ? rgb(profile.rim) : paint;
    const z = profile.pipe ? flowHousing(profile, mesh) : 0;
    const radius = Math.min(w, h) * (profile.radius || 0.065);
    let shape = profile.round ? circle.map(([x, y]) => [(x * w) / 2, (y * h) / 2]) : outline(w, h, radius);
    if (profile.taper) {
        shape = shape.map(([x, y]) => [x * (1 - profile.taper * Math.max(0, (-2 * y) / h)), y]);
    }
    let centerY = 0;
    if (profile.notch) {
        centerY = -h * 0.115;
        shape = outline(w, h * 0.77, h * 0.19);
        for (const side of [-1, 1]) {
            box(w * 0.27, h * 0.34, depth * 0.96, side * w * 0.34, h * 0.33, z, paint, h * 0.075);
        }
    }
    if (profile.ribs) {
        centerY = -h * 0.045;
        shape = outline(w, h * 0.9, radius);
        for (let i = -2; i <= 2; i++) {
            box(w * 0.031, h * 0.13, depth * 0.77, i * w * 0.145, h * 0.435, z, rim, 0.014);
        }
    }
    if (profile.footTerminal) {
        box(w * 0.2, 0.2, depth * 0.52, -w * 0.29, -h / 2 - 0.09, 0, [91, 123, 99], 0.012);
    }
    if (profile.topLug) {
        const lug = profile.topLug;
        box(w * 0.24, lug * 2, depth * 0.12, 0, h / 2, -depth * 0.39, rim, lug * 0.7);
        solid(
            circle,
            [0, h / 2 + lug * 0.48, -depth * 0.32],
            'z',
            [
                [0, lug * 0.24],
                [0.003, lug * 0.24]
            ],
            graphite
        );
    }
    if (profile.mountingPlate) {
        const plate = profile.mountingPlate;
        box(w * 0.65, h * 0.71, plate, 0, 0, z - (depth + plate) / 2, steel, 0.035);
    }
    const position = [0, centerY, z];
    solid(
        shape,
        position,
        'z',
        [
            [-depth / 2, 0.967],
            [-depth * 0.45, 0.985],
            [depth * 0.15, 1.009]
        ],
        rim
    );
    solid(
        shape,
        position,
        'z',
        [
            [depth * 0.149, 1.009],
            [depth * 0.17, 1.002]
        ],
        steel
    );
    solid(
        shape,
        position,
        'z',
        [
            [depth * 0.17, 1.002],
            [depth * 0.185, 1.009],
            [depth * 0.465, 1.009],
            [depth / 2, 1]
        ],
        paint,
        fullFace
    );

    if (profile.wingSpan > w + 0.03) {
        box(profile.wingSpan, h * 0.28, depth * 0.18, 0, 0, -depth * 0.3, rim, 0.055);
    }
    for (const x of profile.glands || []) {
        const r = profile.glandRadius,
            len = profile.glandLength;
        pipeShell(
            face,
            'y',
            [x, -h / 2, 0],
            [
                [-len, r * 0.74],
                [-len * 0.77, r],
                [-len * 0.58, r, true],
                [-len * 0.27, r, true],
                [-len * 0.22, r * 0.77],
                [0.025, r * 0.77]
            ],
            r * 0.42,
            steel
        );
    }
    if (profile.probes) temperatureProbes(w, h, mesh);
    if (profile.cord) {
        const [y, tipX, tipY] = profile.cord;
        const startX = Math.sign(tipX) * w * 0.49;
        const length = tipX - startX;
        lead(
            [startX, y, 0],
            [startX + length * 0.4, y, 0],
            [tipX - length * 0.2, tipY, -0.03],
            [tipX, tipY, -0.03],
            0.038,
            steel
        );
    }
    if (profile.knockouts) {
        for (const y of [-h * 0.235, h * 0.235]) {
            solid(
                circle,
                [w * 0.499, y, 0],
                'x',
                [
                    [-0.01, 0.108],
                    [0.013, 0.108],
                    [0.026, 0.089]
                ],
                rim
            );
        }
        box(w * 0.24, 0.145, 0.071, w * 0.51, 0, -depth * 0.43, rim, 0.045);
    }
}
