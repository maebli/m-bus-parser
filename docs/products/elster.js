import { pipeShell, screwThread } from './pipe.js';

function portOpening(face, outline, center, y, paint) {
    const half = [];
    let a = outline.at(-1);
    for (const b of outline) {
        if (a[0] * center > 0 !== b[0] * center > 0) {
            const t = a[0] / (a[0] - b[0]);
            half.push([0, a[1] + t * (b[1] - a[1])]);
        }
        if (b[0] * center >= 0) half.push(b);
        a = b;
    }
    const edge = [];
    for (let i = 0; i < half.length; i++) {
        const a = half[i],
            b = half[(i + 1) % half.length];
        const steps = Math.ceil(Math.hypot(b[0] - a[0], b[1] - a[1]) / 0.043);
        for (let k = 0; k < steps; k++)
            edge.push([a[0] + ((b[0] - a[0]) * k) / steps, a[1] + ((b[1] - a[1]) * k) / steps]);
    }
    const inner = edge.map(([x, v]) => {
        const r = 0.162 / Math.hypot(x - center, v);
        return [center + (x - center) * r, y, -v * r];
    });
    const outer = edge.map(([x, v]) => [x, y, -v]);
    for (let i = 0; i < edge.length; i++) {
        const j = (i + 1) % edge.length;
        face([outer[i], outer[j], inner[j], inner[i]], paint);
    }
}

export function elsterHousing(face, solid, box, outline, circle) {
    const enamel = [207, 209, 202],
        steel = [157, 163, 161],
        rim = [169, 175, 170];
    const footprint = outline(3.14, 1.36, 0.46, 24);
    solid(
        footprint,
        [0, 0, 0],
        'y',
        [
            [-1.205, 0.91],
            [-1.19, 0.943],
            [-1.152, 0.972],
            [-1.083, 0.988],
            [-0.96, 0.992],
            [-0.082, 1]
        ],
        enamel
    );
    solid(
        footprint,
        [0, 0, 0],
        'y',
        [
            [-0.09, 1],
            [-0.063, 1.025],
            [-0.032, 1.042],
            [0.001, 1.042],
            [0.018, 1.022],
            [0.027, 1]
        ],
        rim
    );
    const roof = 0.99;
    solid(
        footprint,
        [0, 0, 0],
        'y',
        [
            [0.019, 1],
            [0.79, 1],
            [0.895, 0.993],
            [0.949, 0.979],
            [0.978, 0.965],
            [roof, 0.948]
        ],
        enamel,
        null,
        false
    );
    const crown = footprint.map(([x, v]) => [x * 0.948, v * 0.948]);
    for (const x of [-1.25, 1.25]) {
        portOpening(face, crown, x, roof, enamel);
        const collar = [
            [0.79, 0.181],
            [roof, 0.181],
            [0.999, 0.221],
            [1.028, 0.221],
            [1.035, 0.17]
        ];
        collar.push([1.196, 0.17], [1.205, 0.163]);
        pipeShell(face, 'y', [x, 0, 0], collar, 0.139, steel);
        screwThread(face, 'y', [x, 0, 0], 1.038, 1.196, 0.17, 0.182, 7, steel);
        solid(
            circle,
            [x, 0, 0],
            'y',
            [
                [0.741, 0.17],
                [0.747, 0.17]
            ],
            [56, 62, 61]
        );
    }
    const register = outline(1.16, 0.77, 0.085);
    solid(
        register,
        [0, 0.53, 0.652],
        'z',
        [
            [0, 0.9],
            [0.055, 1],
            [0.103, 1],
            [0.119, 0.965]
        ],
        rim
    );
    solid(
        register,
        [0, 0.53, 0.652],
        'z',
        [
            [0.113, 0.965],
            [0.204, 0.958],
            [0.252, 0.922],
            [0.268, 0.897]
        ],
        enamel,
        [0, 0, 1, 1]
    );
    box(0.11, 0.145, 0.087, 0.601, 0.282, 0.757, rim, 0.022);
}

export function elsterIndex(ctx, wheels, id) {
    const { width, height } = ctx.canvas;
    ctx.save();
    ctx.scale(width / 640, height / 360);
    const card = ctx.createLinearGradient(0, 0, 0, 360);
    card.addColorStop(0, '#efefe9');
    card.addColorStop(0.67, '#e5e6de');
    card.addColorStop(1, '#cbd0ca');
    ctx.fillStyle = card;
    ctx.fillRect(0, 0, 640, 360);
    ctx.textBaseline = 'middle';
    ctx.textAlign = 'left';
    ctx.fillStyle = '#47534d';
    ctx.font = '23px monospace';
    ctx.fillText('S/N  ' + (id || '—'), 36, 145);
    ctx.fillStyle = '#323936';
    ctx.fillRect(32, 196, 500, 111);
    for (let i = 0; i < wheels.length; i++) {
        const x = 40 + i * 60 + (i >= 5 ? 6 : 0);
        const fractional = i >= 5;
        const roll = ctx.createLinearGradient(0, 204, 0, 297);
        roll.addColorStop(0, fractional ? '#532522' : '#171a18');
        roll.addColorStop(0.43, fractional ? '#9d4538' : '#434842');
        roll.addColorStop(1, fractional ? '#583027' : '#20241f');
        ctx.fillStyle = roll;
        ctx.fillRect(x, 204, 57, 93);
        ctx.fillStyle = '#f5f4e9';
        ctx.font = 'bold 66px monospace';
        ctx.textAlign = 'center';
        ctx.fillText(wheels[i], x + 29, 252);
    }
    ctx.fillStyle = '#25352e';
    ctx.font = 'bold 39px sans-serif';
    ctx.fillText('m³', 578, 254);
    ctx.fillStyle = '#edece6';
    ctx.font = 'bold 23px monospace';
    ctx.fillText(',', 341, 281);
    ctx.restore();
}
