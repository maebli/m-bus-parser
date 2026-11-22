import { pipeShell } from './pipe.js';

export function g350Housing(face, solid, outline, circle) {
    const enamel = [222, 223, 216],
        cover = [230, 230, 222],
        steel = [148, 154, 152];
    const shell = outline(1.92, 1.12, 0.23, 12);
    const rear = -0.15;
    const roof = 1.09;
    solid(
        shell,
        [0, 0, rear],
        'y',
        [
            [-1.415, 0.9],
            [-1.396, 0.95],
            [-1.355, 0.986],
            [-1.29, 1],
            [0.17, 0.994],
            [0.89, 0.97],
            [1.035, 0.94],
            [roof, 0.895]
        ],
        enamel,
        null,
        false
    );
    const port = [0, roof, rear];
    solid(
        circle,
        port,
        'y',
        [
            [-0.15, 0.233],
            [-0.14, 0.233]
        ],
        [40, 44, 42]
    );
    const rim = shell.map(([u, v]) => [u * 0.895, roof, rear - v * 0.895]);
    const hole = shell.map(([u, v]) => {
        const t = 0.194 / Math.hypot(u, v);
        return [u * t, roof, rear - v * t];
    });
    for (let i = 0; i < rim.length; i++) {
        const j = (i + 1) % rim.length;
        face([rim[i], rim[j], hole[j], hole[i]], enamel);
    }
    pipeShell(
        face,
        'y',
        port,
        [
            [-0.008, 0.248],
            [0.035, 0.256],
            [0.045, 0.225],
            [0.12, 0.225],
            [0.145, 0.307, true],
            [0.279, 0.307, true],
            [0.29, 0.299, true],
            [0.3, 0.31],
            [0.325, 0.31]
        ],
        0.19,
        steel
    );
    pipeShell(
        face,
        'y',
        port,
        [
            [-0.084, 0.115],
            [0.28, 0.115],
            [0.285, 0.109]
        ],
        0.085,
        steel
    );
    solid(
        outline(1.83, 1.44, 0.18, 12),
        [0, 0.365, 0],
        'z',
        [
            [0.15, 0.95],
            [0.28, 1],
            [0.52, 1],
            [0.64, 0.977],
            [0.698, 0.924],
            [0.71, 0.88]
        ],
        cover,
        [0, 0, 1, 0.75]
    );
    const centerY = 0.365;
    for (const [x, color] of [
        [-0.15, [164, 44, 44]],
        [0.053, [46, 50, 49]]
    ]) {
        solid(
            circle,
            [x, centerY - 0.22, 0.71],
            'z',
            [
                [0, 0.08],
                [0.012, 0.081],
                [0.018, 0.075]
            ],
            steel
        );
        solid(
            circle,
            [x, centerY - 0.22, 0.71],
            'z',
            [
                [0.017, 0.069],
                [0.045, 0.067],
                [0.058, 0.056]
            ],
            color
        );
    }
    const lowerLabel = [
        [-0.54, -0.42, 0.414],
        [0.54, -0.42, 0.414],
        [-0.54, -0.88, 0.414]
    ];
    face([lowerLabel[2], [0.54, -0.88, 0.414], lowerLabel[1], lowerLabel[0]], enamel, {
        crop: [0, 0.75, 1, 0.25],
        corners: lowerLabel
    });
}

export function g350Index(ctx, reading, id) {
    ctx.fillStyle = '#e4e4dc';
    ctx.fillRect(0, 0, 640, 480);
    ctx.strokeStyle = '#c5c7be';
    ctx.lineWidth = 2;
    ctx.strokeRect(135, 13, 376, 325);
    ctx.strokeRect(156, 253, 335, 54);
    ctx.textAlign = 'left';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = '#585f59';
    ctx.font = 'bold 20px sans-serif';
    ctx.fillText('G350', 148, 30);
    ctx.font = '15px sans-serif';
    ctx.fillText('Landis+Gyr', 315, 32);
    ctx.fillStyle = '#93988c';
    ctx.fillRect(181, 68, 259, 84);
    const glass = ctx.createLinearGradient(0, 70, 0, 150);
    glass.addColorStop(0, '#849a76');
    glass.addColorStop(1, '#b0bea0');
    ctx.fillStyle = glass;
    ctx.fillRect(187, 74, 247, 72);
    ctx.fillStyle = '#253925';
    ctx.font = '24px monospace';
    ctx.fillText(reading.value, 198, 95, 225);
    ctx.font = '21px monospace';
    ctx.fillText(reading.unit, 198, 126, 225);
    ctx.font = '14px sans-serif';
    ctx.fillStyle = '#666c64';
    ctx.fillText('G4   ·   E6', 158, 324);
    ctx.fillStyle = '#dedfd8';
    ctx.fillRect(0, 360, 640, 120);
    ctx.fillStyle = '#555d57';
    ctx.font = '24px sans-serif';
    ctx.fillText('Landis+Gyr', 36, 388);
    ctx.font = '20px monospace';
    ctx.fillText(id || '', 36, 424, 330);
    ctx.font = '17px sans-serif';
    ctx.fillText('G4    DN 25', 410, 388);
    ctx.fillText('Qmax  6 m³/h', 410, 422);
}
