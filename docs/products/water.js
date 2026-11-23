export function openWaterRegister(face, solid, circle) {
    const cover = [153, 180, 202];
    solid(
        circle,
        [0, 0, 0],
        'y',
        [
            [0.355, 0.435],
            [0.413, 0.438],
            [0.435, 0.418]
        ],
        cover
    );
    solid(
        circle,
        [0, 0, 0],
        'y',
        [
            [0.435, 0.397],
            [0.446, 0.388]
        ],
        [164, 141, 97],
        [0, 0, 1, 1]
    );
    const tilt = 0.36;
    const hingeY = 0.435;
    const hingeZ = -0.405;
    const radius = 0.427;
    const ring = (r, depth) =>
        circle.map(([u, v]) => {
            const y = radius + r * v;
            return [
                r * u,
                hingeY + y * Math.cos(tilt) + depth * Math.sin(tilt),
                hingeZ - y * Math.sin(tilt) + depth * Math.cos(tilt)
            ];
        });
    const rings = [
        [0.39, -0.049],
        [radius, -0.028],
        [radius, 0.043],
        [0.399, 0.049],
        [0.394, 0.012],
        [0.365, -0.003]
    ].map(([r, d]) => ring(r, d));
    face([...rings[0]].reverse(), cover);
    for (let n = 1; n < rings.length; n++) {
        const prior = rings[n - 1],
            next = rings[n];
        for (let i = 0; i < circle.length; i++) {
            const j = (i + 1) % circle.length;
            face([prior[i], prior[j], next[j], next[i]], cover);
        }
    }
    face(rings.at(-1), [184, 201, 212]);
    for (const [start, end] of [
        [-0.17, -0.09],
        [-0.081, 0.081],
        [0.09, 0.17]
    ]) {
        solid(
            circle,
            [0, hingeY, hingeZ],
            'x',
            [
                [start, 0.051],
                [end, 0.051]
            ],
            cover
        );
    }
    solid(
        circle,
        [0, hingeY, hingeZ],
        'x',
        [
            [-0.177, 0.018],
            [0.177, 0.018]
        ],
        [114, 120, 124]
    );
}

export function waterDial(ctx, wheels, id, volume, { encoder, fabricationNumber }) {
    ctx.fillStyle = '#dddcd1';
    ctx.fillRect(0, 0, 512, 512);
    const circle = (x, y, r, color) => {
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.arc(x, y, r, 0, 2 * Math.PI);
        ctx.fill();
    };
    circle(256, 256, 253, '#627d94');
    circle(256, 256, 239, '#eeeede');
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillStyle = '#253943';
    ctx.font = 'bold 31px sans-serif';
    if (encoder) {
        ctx.fillStyle = '#263e64';
        ctx.fillRect(99, 104, 314, 26);
        ctx.fillStyle = '#253943';
        ctx.fillText('m³', 256, 76);
    } else {
        ctx.fillText('m³', 258, 110);
    }
    ctx.fillStyle = '#55564d';
    ctx.fillRect(33, 158, 446, 80);
    for (let i = 0; i < wheels.length; i++) {
        const x = 41 + i * 54;
        const wheel = ctx.createLinearGradient(0, 166, 0, 231);
        const tones = i < 5 ? ['#161c1e', '#3c4342', '#141819'] : ['#631a1e', '#9b3b3a', '#641e23'];
        tones.forEach((tone, j) => wheel.addColorStop(j / 2, tone));
        ctx.fillStyle = wheel;
        ctx.fillRect(x, 165, 51, 66);
        ctx.fillStyle = '#fffdf3';
        ctx.font = 'bold 53px monospace';
        ctx.fillText(wheels[i], x + 25, 201);
    }
    circle(190, 326, 63, '#b3afa0');
    circle(190, 326, 57, '#f7f5e8');
    for (let i = 0; i < 50; i++) {
        const a = (i * Math.PI) / 25;
        const tick = i % 5 === 0 ? 43 : 50;
        ctx.beginPath();
        ctx.moveTo(190 + tick * Math.sin(a), 326 - tick * Math.cos(a));
        ctx.lineTo(190 + 55 * Math.sin(a), 326 - 55 * Math.cos(a));
        ctx.strokeStyle = '#454b48';
        ctx.lineWidth = i % 5 === 0 ? 3 : 1;
        ctx.stroke();
    }
    if (typeof volume === 'number' && Number.isFinite(volume)) {
        const angle = ((volume * 1000) % 1) * 2 * Math.PI;
        ctx.save();
        ctx.translate(190, 326);
        ctx.rotate(angle);
        ctx.beginPath();
        ctx.moveTo(-5, 8);
        ctx.lineTo(0, -46);
        ctx.lineTo(5, 8);
        ctx.fillStyle = '#914133';
        ctx.fill();
        ctx.restore();
    }
    circle(190, 326, 9, '#813b33');
    ctx.fillStyle = '#364341';
    ctx.font = '19px sans-serif';
    ctx.fillText('× 0.1 L', 189, 407);
    ctx.font = '20px monospace';
    ctx.fillText(fabricationNumber || id || '', 331, 345, 135);
    if (fabricationNumber) ctx.fillRect(263, 363, 136, 4);
}
