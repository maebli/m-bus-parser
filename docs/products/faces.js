import { a230Display } from './a230.js';
import { drawDigit } from './digits.js';
import { elsterIndex } from './elster.js';
import { g350Index } from './g350.js';
import { iperlDisplay } from './iperl.js';
import { caseWhite, profiles } from './profiles.js';
import { waterDial } from './water.js';

const sizes = {
    'gmc-emmod206': [600, 750],
    'sensus-iperl': [600, 800],
    'elster-gas': [640, 360],
    'generic-gas': [640, 360],
    'landisplusgyr-g350': [640, 480]
};

export function faceSize(asset) {
    const profile = profiles[asset];
    if (!profile) return sizes[asset] || [512, 512];
    const [w, h] = profile.size;
    const faceHeight = h * (profile.fascia || 1);
    const scale = 640 / Math.max(w, faceHeight);
    return [Math.round(w * scale), Math.round(faceHeight * scale)];
}

function display(ctx, screen, reading, backlight) {
    const { width, height } = ctx.canvas;
    const [x, y, w, h] = screen.map((part, i) => part * (i % 2 ? height : width));
    const frame = Math.min(w, h) * 0.065;
    ctx.fillStyle = '#58605a';
    ctx.fillRect(x - frame, y - frame, w + 2 * frame, h + 2 * frame);
    const glass = ctx.createLinearGradient(x, y, x, y + h);
    glass.addColorStop(0, backlight ? '#8bb6d3' : '#a5b59c');
    glass.addColorStop(1, backlight ? '#d0dfeb' : '#c9d1b4');
    ctx.fillStyle = glass;
    ctx.fillRect(x, y, w, h);
    const [value, time] = reading.value.split(/T(?=\d\d:)/);
    const unit = time || reading.unit;
    ctx.fillStyle = ctx.strokeStyle = '#29352a';
    if (/^-?\d+(\.\d+)?$/.test(value)) {
        const advance = [...value].reduce((sum, char) => sum + (char === '.' ? 10 : 36), 0);
        const scale = Math.min((w * 0.9) / advance, (h * (unit ? 0.53 : 0.72)) / 48);
        ctx.save();
        ctx.translate(x + w - w * 0.05 - advance * scale, y + (h * (unit ? 0.7 : 1) - 48 * scale) / 2);
        ctx.scale(scale, scale);
        ctx.lineWidth = 4.3;
        ctx.lineCap = 'square';
        for (const char of value) {
            if (char === '.') {
                ctx.fillRect(1, 43, 5, 5);
                ctx.translate(10, 0);
            } else {
                drawDigit(ctx, char);
                ctx.translate(36, 0);
            }
        }
        ctx.restore();
    } else {
        ctx.textAlign = 'center';
        ctx.font = h * 0.48 + 'px monospace';
        ctx.fillText(value, x + w * 0.5, y + h * (unit ? 0.36 : 0.5), w * 0.92);
    }
    ctx.textAlign = 'right';
    ctx.font = h * 0.26 + 'px sans-serif';
    ctx.fillText(unit, x + w * 0.94, y + h * 0.81, w * 0.88);
}

function panel(ctx, profile, reading, id) {
    const { width, height } = ctx.canvas;
    ctx.fillStyle = profile.face || caseWhite;
    ctx.fillRect(0, 0, width, height);
    ctx.textBaseline = 'middle';
    if (profile.screen && !profile.round) {
        const inset = Math.min(width, height) * 0.025;
        ctx.strokeStyle = '#a8aea6';
        ctx.lineWidth = inset * 0.24;
        ctx.beginPath();
        ctx.roundRect(inset, inset, width - inset * 2, height - inset * 2, inset * 1.7);
        ctx.stroke();
    }
    if (profile.sideGrips) {
        ctx.fillStyle = '#79818a';
        for (const x of [0.04, 0.89]) {
            ctx.beginPath();
            ctx.roundRect(width * x, height * 0.18, width * 0.07, height * 0.62, width * 0.025);
            ctx.fill();
        }
    }
    if (profile.strip) {
        ctx.fillStyle = profile.strip;
        ctx.fillRect(0, height * 0.14, width, height * 0.045);
    }
    if (profile.panel) {
        const [x, y, w, h, color] = profile.panel;
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.roundRect(width * x, height * y, width * w, height * h, height * 0.04);
        ctx.fill();
    }
    if (profile.ribbedRight) {
        ctx.fillStyle = '#b8b9b2';
        for (let y = height * 0.03; y < height; y += height * 0.043) {
            ctx.fillRect(width * 0.6, y, width * 0.38, height * 0.009);
        }
        ctx.fillStyle = '#d6d7cc';
        ctx.fillRect(width * 0.65, height * 0.16, width * 0.31, height * 0.65);
        ctx.strokeStyle = '#989d94';
        ctx.lineWidth = width * 0.003;
        ctx.strokeRect(width * 0.65, height * 0.16, width * 0.31, height * 0.65);
        ctx.fillStyle = '#a4aaa5';
        ctx.fillRect(width * 0.15, height * 0.847, width * 0.088, height * 0.055);
        ctx.fillStyle = '#50595a';
        for (const x of [0.172, 0.211]) {
            ctx.beginPath();
            ctx.arc(width * x, height * 0.874, height * 0.011, 0, 2 * Math.PI);
            ctx.fill();
        }
    }
    if (profile.terminalCover) {
        ctx.fillStyle = '#bec3bc';
        ctx.fillRect(0, height * 0.66, width, height * 0.34);
    }
    if (profile.screen) display(ctx, profile.screen, reading, profile.backlight);
    for (const [x, y, color, symbol] of profile.keys || []) {
        const r = Math.min(width, height) * 0.068;
        ctx.fillStyle = '#919995';
        ctx.beginPath();
        ctx.arc(width * x, height * y, r * 1.18, 0, 2 * Math.PI);
        ctx.fill();
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.arc(width * x, height * y, r, 0, 2 * Math.PI);
        ctx.fill();
        if (symbol) {
            ctx.textAlign = 'center';
            ctx.font = r * 1.15 + 'px sans-serif';
            ctx.fillStyle = '#eef0e2';
            ctx.fillText(symbol, width * x, height * y);
        }
    }
    if (profile.knockouts || profile.cord) {
        const centers = profile.knockouts ? [0.13, 0.87] : [0.7];
        for (const x of centers) {
            const r = height * (profile.knockouts ? 0.048 : 0.084);
            ctx.beginPath();
            ctx.arc(width * x, height / 2, r, 0, 2 * Math.PI);
            ctx.fillStyle = '#898f89';
            ctx.fill();
            ctx.fillStyle = '#414946';
            ctx.fillRect(width * x - r * 0.65, height / 2 - r * 0.12, r * 1.3, r * 0.24);
        }
    }
    if (profile.mount === 'sensor') {
        ctx.fillStyle = '#858c8b';
        for (const top of [0.13, 0.66]) {
            for (let row = 0; row < 5; row++) {
                for (let column = 0; column < 16; column++) {
                    ctx.beginPath();
                    ctx.arc(
                        width * (0.16 + column * 0.045),
                        height * (top + row * 0.045),
                        width * 0.011,
                        0,
                        2 * Math.PI
                    );
                    ctx.fill();
                }
            }
        }
    }
    if (id && profile.screen && profile.mount !== 'rail') {
        ctx.textAlign = 'center';
        ctx.font = Math.min(width, height) * 0.05 + 'px monospace';
        ctx.fillStyle = '#4e544f';
        ctx.fillText(
            id,
            width * (profile.ribbedRight ? 0.825 : 0.5),
            height * (profile.ribbedRight ? 0.46 : 0.89),
            width * 0.45
        );
    }
}

export function paintFace(ctx, asset, reading, currentVolume, id) {
    const profile = profiles[asset];
    if (profile) return panel(ctx, profile, reading, id);
    if (asset === 'gmc-emmod206') return a230Display(ctx, reading);
    if (asset === 'sensus-iperl') return iperlDisplay(ctx, reading, id);
    if (asset === 'landisplusgyr-g350') return g350Index(ctx, reading, id);
    if (asset === 'itron-bm-plusm') {
        ctx.fillStyle = '#d4d6ce';
        ctx.fillRect(0, 0, 512, 512);
        ctx.fillStyle = '#52595a';
        ctx.fillRect(177, 124, 158, 35);
        return;
    }
    const volume = reading.unit === 'm³' ? reading.value : currentVolume;
    const match = /^(\d{1,5})(?:\.(\d+))?$/.exec(String(volume));
    const wheels = match
        ? match[1].padStart(5, '0') + (match[2] || '').padEnd(3, '0').slice(0, 3)
        : '————————';
    if (asset === 'elster-gas' || asset === 'generic-gas') return elsterIndex(ctx, wheels, id);
    waterDial(ctx, wheels, id, match ? Number(volume) : undefined, {
        encoder: asset === 'gwf-mtkcoder',
        fabricationNumber: reading.fabricationNumber
    });
}
