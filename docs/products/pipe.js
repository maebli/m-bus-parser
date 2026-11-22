const segments = 48;
const angles = Array.from({ length: segments }, (_, i) => (2 * Math.PI * i) / segments);

function ringPoint(axis, center, along, u, v) {
    const [x, y, z] = center;
    return axis === 'x' ? [x + along, y + v, z - u] : [x + u, y + along, z - v];
}

export function pipeShell(face, axis, center, profile, bore, metal) {
    const start = profile[0][0],
        end = profile.at(-1)[0];
    const rings = [[start, bore], ...profile, [end, bore], [start, bore]].map(([along, radius, hex]) =>
        angles.map((angle) => {
            const r = hex
                ? (radius * Math.cos(Math.PI / 6)) / Math.cos((angle % (Math.PI / 3)) - Math.PI / 6)
                : radius;
            return ringPoint(axis, center, along, r * Math.cos(angle), r * Math.sin(angle));
        })
    );
    const inner = metal.map((channel) => Math.round(channel * 0.44));
    for (let j = 1; j < rings.length; j++) {
        for (let i = 0; i < segments; i++) {
            const k = (i + 1) % segments;
            face(
                [rings[j - 1][i], rings[j - 1][k], rings[j][k], rings[j][i]],
                j === rings.length - 1 ? inner : metal
            );
        }
    }
}

export function screwThread(face, axis, center, start, end, root, crest, turns, color) {
    const pitch = (end - start) / (turns + 1);
    const steps = turns * segments;
    let last;
    for (let i = 0; i <= steps; i++) {
        const t = i / steps,
            angle = 2 * Math.PI * turns * t;
        const along = start + pitch * 0.5 + t * (end - start - pitch);
        const height = (crest - root) * Math.min(1, i / 12, (steps - i) / 12);
        const point = (radius, d) =>
            ringPoint(axis, center, along + d, radius * Math.cos(angle), radius * Math.sin(angle));
        const ridge = [
            point(root * 0.996, -pitch * 0.43),
            point(root + height, 0),
            point(root * 0.996, pitch * 0.43)
        ];
        if (last)
            for (let j = 0; j < 2; j++) {
                face([last[j], ridge[j], ridge[j + 1]], color);
                face([last[j], ridge[j + 1], last[j + 1]], color);
            }
        last = ridge;
    }
}

function flange(face, position, y, length, tubeRadius, flangeRadius) {
    const color = [67, 71, 71],
        holeColor = [47, 51, 51];
    const along = (u, v, x) => [x, y + v, -u];
    for (let sector = 0; sector < 4; sector++) {
        const angle = ((sector + 0.5) * Math.PI) / 2;
        const boltX = Math.cos(angle) * flangeRadius * 0.72,
            boltY = Math.sin(angle) * flangeRadius * 0.72;
        const arc = (radius, t) => [radius * Math.cos(t), radius * Math.sin(t)];
        const section = angles.slice(0, 13).map((_, i) => (sector * Math.PI) / 2 + (i * Math.PI) / 24);
        const perimeter = [
            ...section.map((t) => arc(flangeRadius, t)),
            ...section.reverse().map((t) => arc(tubeRadius, t))
        ];
        const bolt = perimeter.map(([u, v]) => {
            const factor = (flangeRadius * 0.105) / Math.hypot(u - boltX, v - boltY);
            return [boltX + (u - boltX) * factor, boltY + (v - boltY) * factor];
        });
        function ring(points, x) {
            return points.map(([u, v]) => along(u, v, x));
        }
        const outerFront = ring(perimeter, position + length),
            outerBack = ring(perimeter, position);
        const holeFront = ring(bolt, position + length),
            holeBack = ring(bolt, position);
        for (let i = 0; i < perimeter.length; i++) {
            const k = (i + 1) % perimeter.length;
            face([outerFront[i], outerFront[k], holeFront[k], holeFront[i]], color);
            face([outerBack[k], outerBack[i], holeBack[i], holeBack[k]], color);
            face([holeBack[k], holeBack[i], holeFront[i], holeFront[k]], holeColor);
            if (i !== 12 && k !== 0) face([outerBack[i], outerBack[k], outerFront[k], outerFront[i]], color);
        }
    }
}

export function flowBody(face, left, right, y, radius, metal, flanged = false, chamber = 1.55) {
    const span = right - left,
        middle = (left + right) / 2;
    const fitting = Math.min(span * 0.105, radius * 1.65);
    let profile;
    if (flanged) {
        const width = Math.min(span * 0.073, radius * 0.55),
            rim = radius * 3.25;
        flange(face, left, y, width, radius, rim);
        flange(face, right - width, y, width, radius, rim);
        profile = [
            [left, radius],
            [left + width, radius],
            [middle, radius * 1.13],
            [right - width, radius],
            [right, radius]
        ];
    } else {
        const end = [
            [0, radius * 0.95],
            [fitting * 0.045, radius]
        ];
        for (const start of [left, right - fitting])
            screwThread(face, 'x', [0, y, 0], start, start + fitting, radius, radius * 1.075, 6, metal);
        end.push(
            [fitting, radius],
            [fitting * 1.05, radius * 1.23, true],
            [fitting * 1.54, radius * 1.23, true],
            [fitting * 1.61, radius]
        );
        profile = [
            ...end.map(([d, r, hex]) => [left + d, r, hex]),
            [middle - span * 0.16, radius * chamber],
            [middle + span * 0.16, radius * chamber],
            ...end.reverse().map(([d, r, hex]) => [right - d, r, hex])
        ];
    }
    pipeShell(face, 'x', [0, y, 0], profile, radius * 0.72, metal);
}
