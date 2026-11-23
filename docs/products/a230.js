import { drawDigit } from './digits.js';

export function a230Housing(face, solid, box, outline, circle) {
    const graphite = [57, 61, 62],
        moulding = [100, 101, 96],
        metal = [163, 164, 154];
    solid(
        outline(136, 136, 2),
        [0, 0, 0],
        'z',
        [
            [-36, 0.9],
            [-33, 0.95],
            [-19, 0.97],
            [-16, 1],
            [0, 1]
        ],
        moulding
    );
    solid(
        outline(144, 144, 1.6),
        [0, 0, 0],
        'z',
        [
            [-1, 0.99],
            [0, 1],
            [3.8, 1],
            [5, 0.985]
        ],
        graphite
    );
    box(137, 137, 0.65, 0, 0, 5, [46, 65, 75], 1, [0, 0, 1, 0.8]);

    function screw(x, y, z, r) {
        solid(
            circle,
            [x, y, z],
            'z',
            [
                [0, r * 0.8],
                [0.4, r],
                [1, r]
            ],
            metal
        );
        box(r * 1.25, r * 0.22, 0.2, x, y, z - 0.05, graphite, 0.1);
    }
    function terminalBlock(x, y, count, pitch, back, front, color) {
        box(count * pitch, 12, front - back, x, y, (front + back) / 2, color, 0.8);
        for (let i = 0; i < count; i++) {
            const pin = x + (i - (count - 1) / 2) * pitch;
            box(pitch * 0.72, 8, 0.7, pin, y, back, graphite, 0.7);
            screw(pin, y, back - 1.1, pitch * 0.23);
        }
    }
    for (const side of [-1, 1]) {
        box(4, 12, 37, side * 68, 0, -16, graphite, 0.6);
        box(6, 18, 5, side * 72, 0, -10, metal, 0.8);
    }
    terminalBlock(0, 51, 10, 10, -46, -31, graphite);
    terminalBlock(-39, -48, 3, 10, -46, -31, graphite);
    terminalBlock(39, -48, 3, 10, -46, -31, graphite);
    box(88, 49, 10, 0, 1, -40, moulding, 1);
    box(88, 49, 18, 0, 1, -54, [68, 74, 74], 1.4);
    for (const x of [-39, 39]) for (const y of [-18, 20]) screw(x, y, -63.7, 2.3);
    terminalBlock(-12, -24, 3, 5, -65, -47, [79, 134, 100]);
    terminalBlock(-26, -24, 2, 5, -65, -47, [79, 134, 100]);
    const corners = [
        [29, 16, -63.2],
        [-29, 16, -63.2],
        [29, -5, -63.2]
    ];
    face([corners[2], [-29, -5, -63.2], corners[1], corners[0]], moulding, {
        crop: [0, 0.8, 1, 0.2],
        corners
    });
}

function numericRows(reading) {
    if (reading.phases) return reading.phases.map((v) => (v == null ? '' : String(v)));
    const value = reading.value;
    if (!/^-?\d+(\.\d+)?$/.test(value)) return ['----', '', ''];
    if (reading.unit === 'kWh') {
        const [whole, fraction = ''] = value.split('.');
        if (whole.startsWith('-') || whole.length + fraction.length > 8) return ['----', '----', ''];
        const digits = whole.padStart(8 - fraction.length, '0') + fraction;
        const split = 8 - fraction.length;
        const row = (start) =>
            [...digits.slice(start, start + 4)]
                .map((d, i) => d + (fraction && start + i + 1 === split ? '.' : ''))
                .join('');
        return [row(0), row(4), ''];
    }
    return [value, '', ''];
}

export function a230Display(ctx, reading) {
    ctx.fillStyle = '#304551';
    ctx.fillRect(0, 0, 600, 600);
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    const rows = numericRows(reading);
    for (let i = 0; i < 3; i++) {
        const y = 83 + 91 * i;
        ctx.fillStyle = '#320f13';
        ctx.fillRect(115, y, 237, 72);
        const chars = rows[i].match(/[-\d]\.?/g) || [];
        const cells = chars.length > 4 ? ['-', '-', '-', '-'] : chars;
        for (let j = 0; j < 4; j++) {
            ctx.save();
            ctx.translate(129 + j * 56, y + 10);
            ctx.transform(1.25, 0, -0.09, 1.1, 0, 0);
            ctx.lineWidth = 5;
            ctx.strokeStyle = '#47171a';
            drawDigit(ctx, '8');
            const char = cells[j - (4 - cells.length)];
            if (char) {
                ctx.strokeStyle = '#ff422d';
                drawDigit(ctx, char[0]);
                if (char.endsWith('.')) {
                    ctx.fillStyle = ctx.strokeStyle;
                    ctx.fillRect(32, 44, 4.6, 4.6);
                }
            }
            ctx.restore();
        }
        ctx.font = 'bold 19px sans-serif';
        const unit = rows[i] ? reading.unit : '';
        const symbols = i === 1 ? ['M', 'k', 'W', 'h'] : ['M', 'k', 'W', i === 2 ? 'Hz' : 'φ'];
        for (const [line, legend] of [symbols, ['V', 'A', 'r', i === 2 ? '%' : 'φ']].entries()) {
            legend.forEach((symbol, k) => {
                ctx.fillStyle = unit.includes(symbol) ? '#ff593c' : '#6b2424';
                ctx.fillText(symbol, 368 + k * 23, y + 23 + line * 29);
            });
        }
    }
    ctx.textAlign = 'center';
    for (const [i, text] of ['P', '↓', '↑'].entries()) {
        const x = 133 + i * 80;
        ctx.fillStyle = '#969b98';
        ctx.beginPath();
        ctx.roundRect(x - 21, 374, 42, 36, 7);
        ctx.fill();
        ctx.fillStyle = '#e5e7dc';
        ctx.font = 'bold 26px sans-serif';
        ctx.fillText(text, x, 393);
    }
    ctx.fillStyle = '#c3c9c8';
    ctx.font = '17px sans-serif';
    ctx.fillText('Display       Clear', 215, 430);
    ctx.fillStyle = '#929895';
    ctx.fillRect(391, 380, 94, 49);
    ctx.fillStyle = '#792b24';
    ctx.font = 'bold 18px sans-serif';
    ctx.fillText('L1  L2  L3', 438, 406);
    ctx.fillStyle = '#dadbd2';
    ctx.fillRect(0, 600, 600, 150);
    ctx.textAlign = 'left';
    ctx.fillStyle = '#343b3d';
    ctx.font = 'bold 31px sans-serif';
    ctx.fillText('EMMOD 206', 23, 643);
    ctx.font = '24px sans-serif';
    ctx.fillText('M-Bus', 24, 703);
}
