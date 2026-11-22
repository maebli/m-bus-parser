const segments = [
    [4, 0, 24, 0],
    [27, 4, 27, 20],
    [27, 27, 27, 43],
    [4, 47, 24, 47],
    [1, 27, 1, 43],
    [1, 4, 1, 20],
    [4, 23.5, 24, 23.5]
];
const digitSegments = [
    '012345',
    '12',
    '01643',
    '01236',
    '5612',
    '05623',
    '054326',
    '012',
    '0123456',
    '012356'
];

export function drawDigit(ctx, char) {
    const active = char === '-' ? '6' : digitSegments[Number(char)];
    for (const index of active) {
        const [x, y, endX, endY] = segments[index];
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(endX, endY);
        ctx.stroke();
    }
}
