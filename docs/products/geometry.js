import { a230Housing } from './a230.js';
import { iperlHousing } from './iperl.js';
import { openWaterRegister } from './water.js';
import { g350Housing } from './g350.js';
import { elsterHousing } from './elster.js';
import { flowBody, pipeShell } from './pipe.js';
import { buildHousing } from './housing.js';

const rail = new Set([
    'abb-delta',
    'berg-dzplus',
    'eastron-sdm630',
    'emh-diz',
    'emu-professional-375',
    'finder-7e-23-8-230-0020',
    'nzr-dhz-5-63',
    'saia-burgess-ale3',
    'generic-electricity'
]);
const inline = {
    'engelmann-sensostar-2': { head: [0.19, 0.02, 0.61, 0.96], pipeY: 0.54, pipeRadius: 0.14 },
    'engelmann-waterstar': { head: [0.261, 0.01, 0.477, 0.98], pipeY: 0.53, pipeRadius: 0.15 },
    'itron-cf-echo-ii': {
        head: [0.522, 0.2, 0.3, 0.76],
        pipeY: 0.45,
        pipeRadius: 0.13,
        ends: [0.17, 0.66],
        flange: true
    },
    'sensus-pollustat': { head: [0.121, 0.054, 0.855, 0.884], pipeY: 0.52, pipeRadius: 0.15 },
    'sensus-pollustat-e': {
        head: [0.282, 0.017, 0.528, 0.713],
        pipeY: 0.34,
        pipeRadius: 0.075,
        ends: [0.163, 0.928]
    },
    'itron-integral-mk-maxx': { head: [0.191, 0.018, 0.61, 0.963], pipeY: 0.59, pipeRadius: 0.145 },
    'landisplusgyr-ultraheat-t230': {
        head: [0.318, 0.166, 0.516, 0.805],
        pipeY: 0.56,
        pipeRadius: 0.085,
        ends: [0.065, 0.55],
        composite: true
    },
    'metrona-ultraheat-xs': { head: [0.186, 0.021, 0.632, 0.96], pipeY: 0.35, pipeRadius: 0.092 },
    'minol-minocal-c2': {
        head: [0.188, 0.012, 0.62, 0.969],
        pipeY: 0.42,
        pipeRadius: 0.108,
        color: [173, 37, 42]
    },
    'sensus-pollucom-e': { head: [0.22, 0.015, 0.568, 0.974], pipeY: 0.64, pipeRadius: 0.12 },
    'siemens-wfh21': { head: [0.22, 0.048, 0.554, 0.751], pipeY: 0.855, pipeRadius: 0.068 }
};

const cableEnclosures = {
    'allmess-cf50': { body: [0.097, 0.016, 0.802, 0.823], glands: [0.24, 0.41, 0.585, 0.754] },
    'itron-cf-51': { body: [0.12, 0.031, 0.755, 0.79], glands: [0.27, 0.42, 0.565, 0.712] },
    'itron-cf-55': { body: [0.117, 0.012, 0.766, 0.838], glands: [0.247, 0.372, 0.502, 0.632, 0.754] },
    'sontex-supercal-531': { body: [0.084, 0.012, 0.831, 0.818], glands: [0.253, 0.414, 0.581, 0.75] },
    'minol-minocal-wr3': { body: [0.12, 0.012, 0.76, 0.853], glands: [0.3, 0.433, 0.569, 0.704] }
};
function outline(w, h, radius, steps = 8) {
    const points = [];
    const r = Math.min(radius, w / 2, h / 2);
    for (let corner = 0; corner < 4; corner++) {
        const x = (corner === 0 || corner === 3 ? 1 : -1) * (w / 2 - r);
        const y = (corner < 2 ? 1 : -1) * (h / 2 - r);
        for (let i = 0; i <= steps; i++) {
            const a = ((corner + i / steps) * Math.PI) / 2;
            points.push([x + r * Math.cos(a), y + r * Math.sin(a)]);
        }
    }
    return points;
}

const circle = Array.from({ length: 32 }, (_, i) => [
    Math.cos((i * Math.PI) / 16),
    Math.sin((i * Math.PI) / 16)
]);
export function meterGeometry(asset, image, bounds) {
    const faces = [];
    const plastic = [207, 208, 198],
        base = [155, 160, 153],
        seam = [100, 107, 102];
    const brass = [191, 145, 72],
        blue = [42, 102, 176];
    function face(points, color, texture) {
        const [a, b, c] = points;
        const u = b.map((v, i) => v - a[i]),
            v = c.map((n, i) => n - a[i]);
        const n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        const length = Math.hypot(...n);
        if (length < 1e-10) return;

        faces.push({ points, normal: n.map((value) => value / length), color, texture });
    }
    function solid(shape, position, axis, rings, color, crop, closeTop = true) {
        const [x, y, z] = position;
        function point(u, v, d) {
            if (axis === 'x') return [x + d, y + v, z - u];
            if (axis === 'y') return [x + u, y + d, z - v];
            return [x + u, y + v, z + d];
        }
        const loops = rings.map(([d, sx, sy = sx]) => shape.map(([a, b]) => point(a * sx, b * sy, d)));
        face([...loops[0]].reverse(), color);
        for (let j = 1; j < loops.length; j++) {
            const prev = loops[j - 1],
                next = loops[j];
            for (let i = 0; i < prev.length; i++) {
                const k = (i + 1) % prev.length;
                face([prev[i], prev[k], next[k], next[i]], color);
            }
        }
        const last = loops.at(-1),
            [depth, sx, sy = sx] = rings.at(-1);
        const width = Math.max(...shape.map((p) => p[0])) * sx;
        const height = Math.max(...shape.map((p) => p[1])) * sy;
        const texture = crop && {
            crop,
            corners: [
                point(-width, height, depth),
                point(width, height, depth),
                point(-width, -height, depth)
            ]
        };
        if (closeTop) face(last, color, texture);
    }
    function box(w, h, d, x, y, z, color, r = 0.06, crop) {
        const bevel = Math.min(0.025, d / 4);
        solid(
            outline(w, h, r),
            [x, y, z],
            'z',
            [
                [-d / 2, 0.96],
                [-d / 2 + bevel, 1],
                [d / 2 - bevel, 1],
                [d / 2, 0.96]
            ],
            color,
            crop
        );
    }
    function cable(path, radius, color) {
        const cross = (a, b) => [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0]
        ];
        const unit = (vector) => vector.map((value) => value / Math.hypot(...vector));
        let previous, up;
        for (let i = 0; i < path.length; i++) {
            const at = path[i],
                before = path[Math.max(i - 1, 0)],
                after = path[Math.min(i + 1, path.length - 1)];
            const tangent = unit(after.map((v, k) => v - before[k]));
            up ??= Math.abs(tangent[1]) > 0.96 ? [0, 0, 1] : [0, 1, 0];
            const right = unit(cross(up, tangent));
            up = cross(tangent, right);
            const ring = circle
                .filter((_, j) => j % 2 === 0)
                .map(([a, b]) => at.map((v, k) => v + radius * (a * right[k] + b * up[k])));
            if (previous) {
                for (let j = 0; j < ring.length; j++) {
                    const k = (j + 1) % ring.length;
                    face([previous[j], previous[k], ring[k], ring[j]], color);
                }
            } else face([...ring].reverse(), color);
            previous = ring;
        }
        face(previous, color);
    }

    function lead(start, controlA, controlB, end, radius, color) {
        const path = Array.from({ length: 21 }, (_, i) => {
            const t = i / 20,
                u = 1 - t;
            return start.map(
                (v, k) =>
                    u ** 3 * v + 3 * u * u * t * controlA[k] + 3 * u * t * t * controlB[k] + t ** 3 * end[k]
            );
        });
        cable(path, radius, color);
    }
    if (asset === 'sensus-iperl') {
        iperlHousing(face, solid, outline, circle);
    } else if (asset === 'gwf-mtkcoder' || asset === 'generic-water') {
        const gwf = asset === 'gwf-mtkcoder';
        const pipeEnd = gwf ? 1.13 : 0.96;
        flowBody(face, -pipeEnd, pipeEnd, -0.29, 0.183, brass, false, gwf ? 2.1 : 1.18);
        solid(
            circle,
            [0, 0, 0],
            'y',
            [
                [-0.135, 0.34],
                [0.035, 0.47],
                [0.14, 0.47],
                [0.2, 0.415]
            ],
            brass
        );
        solid(
            circle,
            [0, 0, 0],
            'y',
            [
                [0.19, 0.407],
                [0.32, 0.442],
                [0.365, 0.415]
            ],
            plastic,
            gwf ? [0.376, 0.386, 0.24, 0.367] : null
        );
        if (gwf) {
            solid(
                circle,
                [0, 0.366, -0.422],
                'x',
                [
                    [-0.11, 0.046],
                    [0.11, 0.046]
                ],
                blue
            );
            solid(
                circle,
                [0, 0.765, -0.44],
                'z',
                [
                    [-0.039, 0.43],
                    [0.027, 0.43],
                    [0.047, 0.407]
                ],
                blue
            );
        } else {
            openWaterRegister(face, solid, circle);
        }
    } else if (inline[asset]) {
        const { head, pipeY, pipeRadius, ends, flange, composite, color = plastic } = inline[asset];
        const [bx, , bw] = bounds;
        const scale = 2.2 / Math.max(image.width * bw, image.height * bounds[3]);
        const sx = image.width * scale,
            sy = image.height * scale;
        const x = (u) => (u - 0.5) * sx,
            y = (v) => (0.5 - v) * sy;
        const [left, right] = (ends || [bx, bx + bw]).map(x);
        const axisY = y(pipeY),
            radius = pipeRadius * sy;
        const material = composite ? [53, 57, 57] : brass;
        const chamber = flange ? 1.13 : 1.55;
        flowBody(face, left, right, axisY, radius, material, flange, chamber);
        const [u, v, w, h] = head;
        const depth = Math.min(w * sx, h * sy) * 0.33;
        const topMounted = asset === 'siemens-wfh21';
        const rear = radius * (flange ? 3.25 : chamber) + 0.035;
        const front = topMounted ? depth / 2 : rear + depth;
        const lift = topMounted ? axisY + radius * chamber + 0.1 - y(v + h) : 0;
        const saddleX = Math.max(
            left + (right - left) * 0.25,
            Math.min(right - (right - left) * (flange ? 0.12 : 0.25), x(u + w / 2))
        );
        if (topMounted) {
            solid(
                circle,
                [saddleX, axisY, 0],
                'y',
                [
                    [radius * 0.92, radius * 1.45],
                    [radius * chamber + 0.1, radius * 1.2]
                ],
                base
            );
        } else {
            solid(
                circle,
                [saddleX, axisY, 0],
                'z',
                [
                    [radius * 0.92, radius * 1.12],
                    [rear - 0.018, radius * 1.15],
                    [rear + 0.012, radius * 1.32]
                ],
                base
            );
        }
        if (asset === 'sensus-pollustat-e') {
            const wire = [144, 48, 39];
            const coil = Array.from({ length: 121 }, (_, i) => {
                const t = i / 120,
                    angle = 5 * Math.PI * t;
                return [
                    x(0.31 + 0.18 * Math.cos(angle)),
                    y(0.786 + 0.166 * Math.sin(angle)),
                    rear - 0.04 - 0.26 * t
                ];
            });
            lead(
                [x(0.52), y(0.713), rear + 0.12],
                [x(0.53), y(0.74), rear + 0.12],
                [x(0.49), y(0.75), rear - 0.04],
                coil[0],
                0.021,
                wire
            );
            cable(coil, 0.021, wire);
            const tip = [x(0.153), y(0.482), coil.at(-1)[2] - 0.035];
            lead(coil.at(-1), [x(0.13), y(0.69), tip[2]], [x(0.14), y(0.57), tip[2]], tip, 0.021, wire);
            solid(
                circle,
                tip,
                'y',
                [
                    [0, 0.029],
                    [0.11, 0.029],
                    [0.14, 0.023]
                ],
                base
            );
        }
        if (composite) {
            const probe = x(0.208);
            solid(
                circle,
                [probe, axisY, 0],
                'y',
                [
                    [radius * 0.9, radius * 0.57],
                    [radius * 1.98, radius * 0.57],
                    [radius * 2.1, radius * 0.37]
                ],
                material
            );
            lead(
                [probe, axisY + radius * 2.07, 0],
                [probe, y(-0.065), 0],
                [x(0.47), y(-0.09), rear],
                [x(0.515), y(0.18), rear + 0.015],
                0.029,
                base
            );
        }
        if (flange) {
            solid(
                circle,
                [x(0.425), axisY, 0],
                'z',
                [
                    [radius * 0.92, radius * 1.45],
                    [radius * 1.38, radius * 1.45]
                ],
                base,
                [0.35, 0.17, 0.15, 0.44]
            );
            lead(
                [x(0.43), axisY - radius * 0.66, radius * 1.32],
                [x(0.405), axisY - radius * 2.4, rear * 0.6],
                [x(0.53), axisY - radius * 2.1, rear],
                [x(0.56), axisY - radius * 1.3, rear + 0.01],
                0.021,
                seam
            );
        }
        buildHousing(
            face,
            image,
            head,
            w * sx,
            h * sy,
            depth,
            x(u + w / 2),
            y(v + h / 2) + lift,
            front,
            color
        );
    } else if (asset === 'itron-bm-plusm') {
        solid(
            circle,
            [0, 0, 0],
            'y',
            [
                [-0.29, 0.84],
                [-0.24, 0.91],
                [0.11, 0.91],
                [0.18, 0.87]
            ],
            [83, 85, 84]
        );
        solid(
            circle,
            [0, 0, 0],
            'y',
            [
                [0.18, 0.84],
                [0.27, 0.8],
                [0.29, 0.78]
            ],
            base,
            [0.24, 0.094, 0.632, 0.46]
        );
        solid(
            circle,
            [0, -0.055, 0.88],
            'z',
            [
                [-0.018, 0.086],
                [0.024, 0.086],
                [0.035, 0.07]
            ],
            base
        );
        box(0.074, 0.012, 0.012, 0, -0.055, 0.916, seam, 0.003);
    } else if (asset === 'gmc-emmod206') {
        a230Housing(face, solid, box, outline, circle);
    } else if (asset === 'elster-gas') {
        elsterHousing(face, solid, box, outline, circle);
    } else if (asset === 'landisplusgyr-g350') {
        g350Housing(face, solid, outline, circle);
    } else if (asset === 'generic-gas') {
        const tankPaint = [181, 184, 177];
        const tank = outline(1.63, 1.03, 0.24);
        solid(
            tank,
            [0, 0, 0],
            'y',
            [
                [-0.67, 0.77],
                [-0.62, 0.92],
                [-0.5, 0.975],
                [-0.09, 1],
                [-0.035, 1.006]
            ],
            tankPaint
        );
        solid(
            tank,
            [0, 0, 0],
            'y',
            [
                [-0.048, 1.015],
                [-0.025, 1.055],
                [0.007, 1.055],
                [0.023, 1.01]
            ],
            plastic
        );
        solid(
            tank,
            [0, 0, 0],
            'y',
            [
                [0.02, 1.004],
                [0.26, 0.965],
                [0.395, 0.918],
                [0.448, 0.85]
            ],
            tankPaint
        );
        const dial = [0.376, 0.166, 0.55, 0.394];
        box(1.0, 0.424, 0.136, 0, 0.203, 0.476, base, 0.09, dial);
        for (const x of [-0.485, 0.485]) {
            solid(
                circle,
                [x, 0, 0],
                'y',
                [
                    [0.4485, 0.101],
                    [0.45, 0.101]
                ],
                [47, 49, 46]
            );
            pipeShell(
                face,
                'y',
                [x, 0, 0],
                [
                    [0.39, 0.155],
                    [0.5, 0.155],
                    [0.52, 0.18, true],
                    [0.6, 0.18, true],
                    [0.61, 0.13],
                    [0.68, 0.13]
                ],
                0.1,
                base
            );
        }
    } else {
        const crop = asset === 'relay-padpuls-2' ? [0.01, 0.06, 0.758, 0.893] : bounds;
        const finder = asset === 'finder-7e-23-8-230-0020',
            eastron = asset === 'eastron-sdm630';
        const aspect = finder
            ? 17.5 / 89.2
            : eastron
              ? 72 / 100
              : (image.width * crop[2]) / (image.height * crop[3]);
        const w = Math.min(1.8, 1.72 * aspect),
            h = w / aspect;
        const isRail = rail.has(asset);
        const depth = isRail
            ? h * (finder ? 58 / 89.2 : eastron ? 0.66 : 0.59)
            : Math.min(0.77, Math.max(0.36, w * 0.38));
        const caseColor = asset === 'itron-cyble-m-bus-v1-4' ? [187, 183, 177] : plastic;
        const sx = w / crop[2],
            sy = h / crop[3];
        const cx = (u) => (u - crop[0]) * sx - w / 2,
            cy = (v) => h / 2 - (v - crop[1]) * sy;
        function enclosurePart(rect, thick, z, material = caseColor) {
            const [u, v, du, dv] = rect;
            buildHousing(
                face,
                image,
                rect,
                du * sx,
                dv * sy,
                thick,
                cx(u + du / 2),
                cy(v + dv / 2),
                z,
                material
            );
        }
        const enclosure = cableEnclosures[asset];
        if (enclosure) {
            const [u, v, du, dv] = enclosure.body;
            enclosurePart(enclosure.body, depth, depth / 2);
            for (const [start, end] of [
                [crop[0], u],
                [u + du, crop[0] + crop[2]]
            ]) {
                if (end - start > 0.004)
                    enclosurePart([start, v, end - start, dv], depth * 0.18, -depth * 0.22);
            }
            for (const gland of enclosure.glands) {
                const radius = w * 0.048,
                    len = Math.max(radius * 0.8, cy(v + dv) - cy(crop[1] + crop[3]));
                pipeShell(
                    face,
                    'y',
                    [cx(gland), cy(v + dv), 0],
                    [
                        [-len, radius * 0.74],
                        [-len * 0.77, radius],
                        [-len * 0.58, radius, true],
                        [-len * 0.27, radius, true],
                        [-len * 0.22, radius * 0.77],
                        [0.025, radius * 0.77]
                    ],
                    radius * 0.42,
                    base
                );
            }
        } else if (asset === 'elster-falcon' || asset === 'itron-cyble-m-bus-v1-4') {
            const falcon = asset === 'elster-falcon';
            const cut = falcon ? 0.195 : 0.854;
            const [u, v, du, dv] = crop;
            enclosurePart(falcon ? [cut, v, u + du - cut, dv] : [u, v, cut - u, dv], depth, depth / 2);
            const start = falcon ? cut : 0.845,
                end = falcon ? u : u + du;
            const a = [cx(start), cy(falcon ? 0.452 : 0.741), 0],
                b = [cx(end), cy(falcon ? 0.445 : 0.886), -0.03];
            lead(
                a,
                [a[0] + (b[0] - a[0]) * 0.4, a[1], 0],
                [b[0] - (b[0] - a[0]) * 0.2, b[1], b[2]],
                b,
                0.038,
                base
            );
        } else if (isRail) {
            const outerHeight = finder ? (85 - 62.3) / 2 / 85 : 0.135;
            const innerHeight = finder ? (85 - 45) / 2 / 85 : eastron ? 0.275 : 0.225;
            const levels = [
                [0, outerHeight, finder ? 24 / 58 : 0.56],
                [outerHeight, innerHeight, finder ? 44 / 58 : 0.79],
                [innerHeight, 1 - innerHeight, 1],
                [1 - innerHeight, 1 - outerHeight, finder ? 44 / 58 : 0.79],
                [1 - outerHeight, 1, finder ? 24 / 58 : 0.56]
            ];
            for (const [lo, hi, level] of levels) {
                const thickness = depth * level;
                const tint =
                    asset === 'emu-professional-375' && lo === 0
                        ? [175, 43, 43]
                        : asset === 'emu-professional-375' && hi === 1
                          ? [55, 95, 154]
                          : caseColor;
                enclosurePart(
                    [crop[0], crop[1] + lo * crop[3], crop[2], (hi - lo) * crop[3]],
                    thickness,
                    thickness - depth / 2,
                    tint
                );
            }
        } else {
            enclosurePart(crop, depth, depth / 2);
        }
        if (asset === 'engelmann-sensostar-2c') {
            for (const [gland, offset] of [
                [0.125, 0.26],
                [0.28, 0]
            ]) {
                const start = [gland * w - w / 2, -h * 0.48, 0];
                const tipX = w / 2 + 0.18 + offset;
                const bottom = -h / 2 - 0.32 - offset * 0.65;
                const bend = [(start[0] + tipX) / 2, bottom, 0];
                const probeY = -h / 2 - 0.04;
                lead(start, [start[0], bottom, 0], [start[0] + 0.12, bottom, 0], bend, 0.014, [53, 53, 50]);
                lead(
                    bend,
                    [tipX - 0.08, bottom, 0],
                    [tipX, bottom, 0],
                    [tipX, probeY, 0],
                    0.014,
                    [53, 53, 50]
                );
                solid(
                    circle,
                    [tipX, probeY, 0],
                    'y',
                    [
                        [-0.01, 0.021],
                        [0.05, 0.027],
                        [0.074, 0.026]
                    ],
                    seam
                );
                pipeShell(
                    face,
                    'y',
                    [tipX, probeY, 0],
                    [
                        [0.062, 0.032],
                        [0.08, 0.046, true],
                        [0.132, 0.046, true],
                        [0.14, 0.035],
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
                    [183, 188, 186]
                );
            }
        }
        if (asset === 'relay-padpuls-2') {
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
                    base
                );
            }
            box(w * 0.24, 0.145, 0.071, w * 0.51, 0, -depth * 0.43, base, 0.045);
        }
        if (isRail) {
            for (const side of [-1, 1]) {
                box(w * 0.87, h * 0.2, 0.16, 0, side * h * 0.29, -depth * 0.55, base, 0.028);
                box(w * 0.81, 0.055, 0.13, 0, side * h * 0.19, -depth * 0.61, seam, 0.015);
            }
        }
    }
    const points = faces.flatMap((f) => f.points);
    const low = [0, 1, 2].map((i) => Math.min(...points.map((p) => p[i])));
    const high = [0, 1, 2].map((i) => Math.max(...points.map((p) => p[i])));
    const center = low.map((v, i) => (v + high[i]) / 2);
    const radius = Math.max(...points.map((p) => Math.hypot(...p.map((v, i) => v - center[i]))));
    return { faces, center, radius };
}
