import { drawDigit } from './digits.js';
import { pipeShell, screwThread } from './pipe.js';

export function iperlHousing(face, solid, outline, circle) {
    const black = [58, 62, 65],
        shell = [203, 204, 194],
        blue = [55, 151, 212];
    const depth = 0.45;
    const bandHeight = (x) => 0.691 + 0.333 * x;
    const sections = [
        [0.31, -0.26],
        [0.354, -0.22],
        [0.354, 0.23],
        [0.41, 0.38],
        [0.43, bandHeight(-0.43), bandHeight(0.43)]
    ];
    const left = sections.map(([width, height]) => [-width, height]);
    const right = sections.map(([width, height, rightHeight = height]) => [width, rightHeight]);
    const side = [...right, ...[...left].reverse()];
    const sideTexture = {
        crop: [0, 0.6, 1, 0.4],
        corners: [
            [-0.43, 0.84, depth],
            [0.43, 0.84, depth],
            [-0.43, -0.26, depth]
        ]
    };
    for (let i = 1; i < sections.length; i++) {
        const strip = [left[i - 1], right[i - 1], right[i], left[i]];
        face(
            strip.map(([x, y]) => [x, y, depth]),
            black,
            sideTexture
        );
        face(
            strip.reverse().map(([x, y]) => [x, y, -depth]),
            black
        );
    }
    for (let i = 0; i < side.length; i++) {
        const next = (i + 1) % side.length;
        const [x, low] = side[i],
            [endX, high] = side[next];
        if (x !== endX) {
            face(
                [
                    [x, low, -depth],
                    [endX, high, -depth],
                    [endX, high, depth],
                    [x, low, depth]
                ],
                black
            );
            continue;
        }
        const bottom = Math.min(low, high),
            top = Math.max(low, high);
        const contour = [
            [-depth, bottom],
            [depth, bottom],
            [depth, top],
            [-depth, top]
        ];
        const rim = [],
            hole = [];
        for (let edge = 0; edge < 4; edge++) {
            const a = contour[edge],
                b = contour[(edge + 1) % 4];
            for (let j = 0; j < 16; j++) {
                const z = a[0] + ((b[0] - a[0]) * j) / 16,
                    y = a[1] + ((b[1] - a[1]) * j) / 16;
                const k = 0.112 / Math.hypot(z, y);
                rim.push([x, y, z]);
                hole.push([x, y * k, z * k]);
            }
        }
        for (let j = 0; j < rim.length; j++) {
            const k = (j + 1) % rim.length,
                patch = [rim[j], rim[k], hole[k], hole[j]];
            face(x < 0 ? patch : patch.reverse(), black);
        }
    }
    pipeShell(
        face,
        'x',
        [0, 0, 0],
        [
            [-0.85, 0.129],
            [-0.83, 0.132],
            [-0.36, 0.132],
            [-0.35, 0.154],
            [0.35, 0.154],
            [0.36, 0.132],
            [0.83, 0.132],
            [0.85, 0.129]
        ],
        0.092,
        black
    );
    for (const start of [-0.845, 0.708])
        screwThread(face, 'x', [0, 0, 0], start, start + 0.137, 0.132, 0.142, 7, black);
    const footprint = outline(0.89, 0.94, 0.064, 10);
    const band = footprint.map(([x, v]) => [x, bandHeight(x), -v]);
    const shoulder = footprint.map(([x, v]) => [x, 0.89 + 0.036 * x, -v]);
    const deckPoint = (x, v) => [x * 0.97, 0.911 + 0.036 * x, -v * 0.98];
    const deck = footprint.map(([x, v]) => deckPoint(x, v));
    for (let i = 0; i < deck.length; i++) {
        const j = (i + 1) % deck.length;
        face([band[i], band[j], shoulder[j], shoulder[i]], shell);
        face([shoulder[i], shoulder[j], deck[j], deck[i]], shell);
    }
    face([...band].reverse(), shell);
    face(deck, shell, {
        crop: [0, 0, 1, 0.6],
        corners: [deckPoint(-0.445, 0.47), deckPoint(0.445, 0.47), deckPoint(-0.445, -0.47)]
    });
    const hingeX = 0.37,
        hingeY = 0.938;
    for (const [start, end] of [
        [-0.391, -0.29],
        [-0.28, 0.28],
        [0.29, 0.391]
    ]) {
        solid(
            circle,
            [hingeX, hingeY, 0],
            'z',
            [
                [start, 0.025],
                [end, 0.025]
            ],
            start === -0.28 ? shell : blue
        );
    }
    const hingeAngle = 2.4;
    const lidRim = Array.from({ length: 49 }, (_, i) => {
        const t = (i / 48 - 0.5) * Math.PI;
        return [0.75 * Math.cos(t), 0.388 * Math.sin(t)];
    });
    const lidSurface = (offset) =>
        lidRim.map(([reach, z]) => [
            hingeX - reach * Math.cos(hingeAngle) + offset * Math.sin(hingeAngle),
            hingeY + reach * Math.sin(hingeAngle) + offset * Math.cos(hingeAngle),
            z
        ]);
    const outside = lidSurface(0.018),
        inside = lidSurface(-0.006);
    face(outside, blue);
    face([...inside].reverse(), [113, 192, 228]);
    for (let i = 0; i < inside.length; i++) {
        const j = (i + 1) % inside.length;
        face([inside[i], inside[j], outside[j], outside[i]], [49, 136, 192]);
    }
}

function lcdValue(ctx, value) {
    ctx.fillStyle = '#253f53';
    ctx.textAlign = 'right';
    if (!/^-?\d+(?:\.\d+)?$/.test(value) || value.length > 11) {
        ctx.font = '46px monospace';
        ctx.fillText(value, 466, 232, 315);
        return;
    }
    const width = [...value].reduce((sum, char) => sum + (char === '.' ? 10 : 36), 0);
    ctx.save();
    ctx.translate(464 - Math.min(width, 315), 207);
    ctx.scale(Math.min(1, 315 / width), 1);
    ctx.transform(1, 0, -0.05, 1, 2, 0);
    ctx.lineWidth = 4.6;
    ctx.strokeStyle = '#253f53';
    for (const char of value) {
        if (char === '.') {
            ctx.fillRect(0, 44, 5, 5);
            ctx.translate(10, 0);
            continue;
        }
        drawDigit(ctx, char);
        ctx.translate(36, 0);
    }
    ctx.restore();
}

export function iperlDisplay(ctx, reading, id) {
    ctx.fillStyle = '#dadbd0';
    ctx.fillRect(0, 0, 600, 480);
    ctx.textBaseline = 'middle';
    ctx.textAlign = 'left';
    ctx.fillStyle = '#464d4c';
    ctx.font = '18px sans-serif';
    ctx.fillText(id || '', 26, 26);
    ctx.beginPath();
    ctx.moveTo(512, 49);
    ctx.bezierCurveTo(255, 51, 57, 135, 49, 241);
    ctx.bezierCurveTo(54, 349, 255, 419, 512, 423);
    ctx.closePath();
    ctx.strokeStyle = '#aaaea4';
    ctx.lineWidth = 4;
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(481, 158);
    ctx.quadraticCurveTo(269, 156, 129, 218);
    ctx.quadraticCurveTo(73, 241, 135, 265);
    ctx.quadraticCurveTo(290, 314, 481, 313);
    ctx.quadraticCurveTo(491, 313, 491, 302);
    ctx.lineTo(491, 169);
    ctx.closePath();
    const glass = ctx.createLinearGradient(0, 155, 0, 315);
    glass.addColorStop(0, '#99a897');
    glass.addColorStop(1, '#c3cfb8');
    ctx.fillStyle = glass;
    ctx.fill();
    ctx.lineWidth = 5;
    ctx.strokeStyle = '#89968b';
    ctx.stroke();
    lcdValue(ctx, reading.value);
    ctx.font = '22px sans-serif';
    ctx.fillText(reading.unit, 464, 279, 100);
    ctx.fillStyle = '#4e5450';
    ctx.beginPath();
    ctx.arc(306, 126, 9, 0, 2 * Math.PI);
    ctx.fill();
    ctx.fillStyle = '#34393c';
    ctx.fillRect(0, 480, 600, 320);
}
