export function buildHousing(face, image, crop, width, height, depth, cx, cy, front, color) {
    const [u, v, uw, vh] = crop;
    const xStart = Math.round(u * image.width),
        xEnd = Math.round((u + uw) * image.width);
    const columns = xEnd - xStart;
    if (!columns) return;
    const rowCount = Math.max(6, Math.round((90 * height) / Math.max(width, height)));
    const worldX = (x) => cx + (x / columns - 0.5) * width;
    const worldY = (row) => cy + (0.5 - row / rowCount) * height;
    const bounds = [
        [cx - width / 2, cy + height / 2, front],
        [cx + width / 2, cy + height / 2, front],
        [cx - width / 2, cy - height / 2, front]
    ];
    const texture = { crop, corners: bounds };
    const back = front - depth;
    const molded = depth > 0.12;
    const rearColor = color.map((n) => Math.round(n * 0.86));
    const joint = color.map((n) => Math.round(n * 0.61));
    const shell = (cap, amount, scale) =>
        cap.map(([x, y]) => [cx + (x - cx) * scale, cy + (y - cy) * scale, front - depth * amount]);

    function scan(row) {
        const sourceY = Math.min(
            image.height - 1,
            Math.floor((v + (vh * (row + 0.5)) / rowCount) * image.height)
        );
        const runs = [];
        let start;
        for (let x = 0; x <= columns; x++) {
            const solid = x < columns && image.data[(sourceY * image.width + xStart + x) * 4 + 3] >= 145;
            if (solid && start === undefined) start = x;
            if (!solid && start !== undefined) {
                if (x - start >= 2) runs.push([worldX(start), worldX(x)]);
                start = undefined;
            }
        }
        return runs;
    }

    function boundary(row, neighbor) {
        if (
            row.length !== neighbor.length ||
            row.some(([a, b], i) => b < neighbor[i][0] || a > neighbor[i][1])
        )
            return row;
        return row.map(([a, b], i) => [(a + neighbor[i][0]) / 2, (b + neighbor[i][1]) / 2]);
    }

    const rows = Array.from({ length: rowCount }, (_, row) => scan(row));
    for (let row = 0; row < rowCount; row++) {
        const runs = rows[row],
            previous = rows[row - 1] || [],
            next = rows[row + 1] || [];
        const top = boundary(runs, previous),
            bottom = boundary(runs, next);
        for (let i = 0; i < runs.length; i++) {
            const [leftTop, rightTop] = top[i],
                [leftBottom, rightBottom] = bottom[i];
            const yTop = worldY(row),
                yBottom = worldY(row + 1);
            const cap = [
                [leftTop, yTop, front],
                [leftBottom, yBottom, front],
                [rightBottom, yBottom, front],
                [rightTop, yTop, front]
            ];
            const rings = molded
                ? [
                      cap,
                      shell(cap, 0.035, 1.009),
                      shell(cap, 0.32, 1.009),
                      shell(cap, 0.335, 1.003),
                      shell(cap, 0.35, 1.009),
                      shell(cap, 0.95, 0.988),
                      shell(cap, 1, 0.968)
                  ]
                : [cap, cap.map(([x, y]) => [x, y, back])];
            face(cap, color, texture);
            face([...rings.at(-1)].reverse(), rearColor);
            const wall = (edge) => {
                const end = (edge + 1) % 4;
                for (let j = 1; j < rings.length; j++) {
                    const a = rings[j - 1],
                        b = rings[j];
                    face([a[edge], b[edge], b[end], a[end]], molded && (j === 3 || j === 4) ? joint : color);
                }
            };
            wall(0);
            wall(2);
            if (bottom === runs) wall(1);
            if (top === runs) wall(3);
        }
    }
}
