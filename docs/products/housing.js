import { flowBody, pipeShell } from './pipe.js';
import { caseWhite } from './profiles.js';

const steel = [139, 146, 145];
const graphite = [59, 65, 66];
const brass = [191, 145, 72];
const fullFace = [0, 0, 1, 1];

function rgb(hex) {
    return hex.match(/[0-9a-f]{2}/gi).map((pair) => parseInt(pair, 16));
}

export function buildHousing(profile, { face, solid, box, outline, circle, lead }) {
    const [w, h, depth] = profile.size.map((mm) => mm / 100);
    const paint = rgb(profile.face || caseWhite);
    const rim = profile.rim ? rgb(profile.rim) : steel;
    const radius = Math.min(w, h) * (profile.radius || 0.065);
    const shape = profile.round ? circle.map(([x, y]) => [(x * w) / 2, (y * h) / 2]) : outline(w, h, radius);

    function gland(x, y, z, radius) {
        pipeShell(
            face,
            'y',
            [x, y, z],
            [
                [-radius * 1.7, radius * 0.78],
                [-radius, radius, true],
                [-radius * 0.4, radius, true],
                [0.02, radius * 0.79]
            ],
            radius * 0.44,
            rim
        );
    }

    if (profile.mount === 'rail') {
        const halfDepth = depth / 2;
        const caseColor = rgb(profile.case || caseWhite);
        box(w, h, depth * 0.52, 0, 0, -depth * 0.24, caseColor, 0.015);
        for (const [index, side] of [1, -1].entries()) {
            const cap = profile.caps ? rgb(profile.caps[index]) : caseColor;
            box(w, h * 0.23, depth * 0.24, 0, side * h * 0.32, depth * 0.13, cap, 0.012);
            for (let i = 0; i < profile.terminals; i++) {
                const x = w * ((i + 0.5) / profile.terminals - 0.5);
                const r = Math.min(0.032, (w / profile.terminals) * 0.22);
                solid(
                    circle,
                    [x, side * h * 0.34, depth * 0.25],
                    'z',
                    [
                        [0, r],
                        [0.003, r]
                    ],
                    graphite
                );
                box(r * 1.35, r * 0.28, 0.004, x, side * h * 0.34, depth * 0.255 + 0.003, steel, 0.001);
            }
            box(w * 0.8, h * 0.13, 0.08, 0, side * (0.175 + h * 0.065), -halfDepth, graphite, 0.01);
        }
        box(w, h * 0.5, depth * 0.46, 0, 0, depth * 0.27, paint, 0.018, fullFace);
        return;
    }

    const headX = profile.flange ? w * 0.74 : 0;
    const pipeRadius = profile.flange ? 0.16 : 0.13;
    const headY = profile.upright ? h / 2 + pipeRadius * 1.4 : 0;
    const headZ = profile.mount === 'pipe' && !profile.upright ? depth / 2 + pipeRadius * 1.55 : 0;

    if (profile.mount === 'pipe') {
        const end = profile.span / 200;
        flowBody(face, -end, end, 0, pipeRadius, profile.composite ? graphite : brass, profile.flange);
        if (profile.flange) {
            solid(
                circle,
                [0, 0, 0],
                'z',
                [
                    [0.1, 0.24],
                    [0.25, 0.24],
                    [0.27, 0.21]
                ],
                steel
            );
            lead(
                [0, -0.12, 0.21],
                [0.1, -0.53, 0.25],
                [0.6, -0.51, headZ],
                [headX, -0.3, headZ],
                0.016,
                graphite
            );
        }
        if (profile.probe || profile.composite) {
            const color = profile.composite ? graphite : [156, 57, 49];
            const side = profile.composite ? 1 : -1;
            const tip = [-end * 0.69, side * (h * 0.54), 0];
            lead(
                [-w * 0.24, side * h * 0.46, headZ],
                [0, side * h * 1.12, headZ],
                [-end * 1.1, side * h, 0],
                tip,
                0.019,
                color
            );
            solid(
                circle,
                tip,
                'y',
                [
                    [-0.14, 0.023],
                    [0.045, 0.023]
                ],
                steel
            );
        }
    }

    solid(
        shape,
        [headX, headY, headZ],
        'z',
        [
            [-depth / 2, 0.91],
            [-depth * 0.37, 1],
            [depth * 0.24, 1],
            [depth * 0.29, 0.98]
        ],
        rim
    );
    solid(
        shape,
        [headX, headY, headZ],
        'z',
        [
            [depth * 0.29, 0.98],
            [depth * 0.44, 1],
            [depth / 2, 0.95]
        ],
        paint,
        fullFace
    );

    if (profile.ears) {
        for (const side of [-1, 1]) {
            box(w * 0.18, h * 0.26, 0.055, side * w * 0.495, 0, -depth * 0.43, rim, 0.045);
            solid(
                circle,
                [side * w * 0.545, 0, -depth * 0.43 + 0.029],
                'z',
                [
                    [0, 0.022],
                    [0.002, 0.022]
                ],
                graphite
            );
        }
    }
    for (let i = 0; i < (profile.glands || 0); i++) {
        gland(
            w * ((i + 0.5) / profile.glands - 0.5) * 0.9,
            -h * 0.48,
            0,
            Math.min(0.059, w / profile.glands / 3)
        );
    }
    if (profile.probes) {
        for (const [i, x] of [-0.37, -0.12].entries()) {
            const probeX = w * (0.62 + i * 0.14);
            lead(
                [x * w, -h * 0.52, 0],
                [x * w, -h * (0.84 + i * 0.12), 0],
                [probeX, -h * 0.77, 0],
                [probeX, -h * 0.46, 0],
                0.014,
                graphite
            );
            gland(probeX, -h * 0.4, 0, 0.036);
            solid(
                circle,
                [probeX, -h * 0.4, 0],
                'y',
                [
                    [0, 0.019],
                    [0.22, 0.019]
                ],
                steel
            );
        }
    }
    if (profile.cord) {
        const side = profile.cord;
        const lug = (-side * w) / 2;
        box(w * 0.21, h * 0.25, 0.035, lug, h * 0.2, -depth * 0.34, rim, 0.023);
        solid(
            circle,
            [lug * 1.1, h * 0.2, -depth * 0.34 + 0.02],
            'z',
            [
                [0, 0.014],
                [0.002, 0.014]
            ],
            graphite
        );
        lead(
            [side * w * 0.46, 0, 0],
            [side * w * 0.72, 0, 0],
            [side * w * 0.77, -0.12, 0],
            [side * w, -0.11, 0],
            0.028,
            rim
        );
    }
    if (profile.knockouts) {
        for (const y of [-h / 4, h / 4]) {
            solid(
                circle,
                [w * 0.5, y, 0],
                'x',
                [
                    [-0.01, 0.08],
                    [0.015, 0.08],
                    [0.017, 0.068]
                ],
                rim
            );
        }
    }
}
