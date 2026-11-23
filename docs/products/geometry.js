import { a230Housing } from './a230.js';
import { iperlHousing } from './iperl.js';
import { openWaterRegister } from './water.js';
import { g350Housing } from './g350.js';
import { elsterHousing } from './elster.js';
import { flowBody, pipeShell } from './pipe.js';
import { profiles } from './profiles.js';
import { buildHousing } from './housing.js';

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
export function meterGeometry(asset) {
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
        const pipeEnd = gwf ? 1.1 : 0.96;
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
            gwf ? [0, 0, 1, 1] : null
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
            [0, 0, 1, 1]
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
        const dial = [0, 0, 1, 1];
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
        buildHousing(profiles[asset], { face, solid, box, outline, circle, lead });
    }
    const points = faces.flatMap((f) => f.points);
    const low = [0, 1, 2].map((i) => Math.min(...points.map((p) => p[i])));
    const high = [0, 1, 2].map((i) => Math.max(...points.map((p) => p[i])));
    const center = low.map((v, i) => (v + high[i]) / 2);
    const radius = Math.max(...points.map((p) => Math.hypot(...p.map((v, i) => v - center[i]))));
    return { faces, center, radius };
}
