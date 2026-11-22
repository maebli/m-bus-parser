function litColor([r, g, b], light, highlight) {
    return (
        (255 << 24) |
        (Math.min(255, Math.round(b * light + highlight)) << 16) |
        (Math.min(255, Math.round(g * light + highlight)) << 8) |
        Math.min(255, Math.round(r * light + highlight))
    );
}

export function textureLevels(surface) {
    const levels = [];
    let canvas = surface;
    while (true) {
        const { width, height } = canvas;
        const pixels = canvas
            .getContext('2d', { willReadFrequently: true })
            .getImageData(0, 0, width, height);
        levels.push({ width, height, colors: new Uint32Array(pixels.data.buffer) });
        if (Math.min(width, height) < 80) {
            if (canvas !== surface) canvas.width = 0;
            return levels;
        }
        const smaller = document.createElement('canvas');
        smaller.width = Math.ceil(width / 2);
        smaller.height = Math.ceil(height / 2);
        const ctx = smaller.getContext('2d', { willReadFrequently: true });
        ctx.drawImage(canvas, 0, 0, smaller.width, smaller.height);
        if (canvas !== surface) canvas.width = 0;
        canvas = smaller;
    }
}

function blend(a, b, weight) {
    const mix = (x, y) => x + (((y - x) * weight) >> 8);
    return (
        0xff000000 |
        (mix((a >> 16) & 255, (b >> 16) & 255) << 16) |
        (mix((a >> 8) & 255, (b >> 8) & 255) << 8) |
        mix(a & 255, b & 255)
    );
}

function texel(pixels, u, v, base) {
    const x = Math.max(0, Math.min(pixels.width - 1, u - 0.5)),
        y = Math.max(0, Math.min(pixels.height - 1, v - 0.5));
    const i = Math.floor(x) + Math.floor(y) * pixels.width;
    const ix = x < pixels.width - 1 ? i + 1 : i,
        stride = y < pixels.height - 1 ? pixels.width : 0;
    const read = (index) => (pixels.colors[index] >>> 24 >= 128 ? pixels.colors[index] : base);
    const fractionX = (x % 1) * 256,
        fractionY = (y % 1) * 256;
    return blend(
        blend(read(i), read(ix), fractionX),
        blend(read(i + stride), read(ix + stride), fractionX),
        fractionY
    );
}

function textureMapping(texture, project, levels) {
    const [origin, right, bottom] = texture.corners.map(project);
    const ax = right[0] - origin[0],
        ay = right[1] - origin[1];
    const bx = bottom[0] - origin[0],
        by = bottom[1] - origin[1];
    const determinant = ax * by - ay * bx;
    const [x, y, width, height] = texture.crop;
    const footprint =
        Math.min(
            Math.hypot(bx, by) * width * levels[0].width,
            Math.hypot(ax, ay) * height * levels[0].height
        ) / Math.abs(determinant);
    const lod = Math.max(0, Math.min(levels.length - 1, Math.floor(Math.log2(footprint))));
    const pixels = levels[lod];
    const uScale = (width * pixels.width) / determinant;
    const vScale = (height * pixels.height) / determinant;
    return {
        origin,
        pixels,
        u: x * pixels.width,
        v: y * pixels.height,
        ux: by * uScale,
        uy: -bx * uScale,
        vx: -ay * vScale,
        vy: ax * vScale
    };
}

export class MeterRasterizer {
    constructor(canvas) {
        this.context = canvas.getContext('2d');
    }

    draw(geometry, texture, width, height, yaw, pitch) {
        const { canvas } = this.context;
        if (canvas.width !== width || canvas.height !== height) {
            canvas.width = width;
            canvas.height = height;
        }
        const size = Math.min(width, height);
        if (this.frame?.width !== size) {
            this.frame = this.context.createImageData(size, size);
            this.colors = new Uint32Array(this.frame.data.buffer);
            this.depth = new Float32Array(size * size);
            this.left = new Float32Array(size);
            this.right = new Float32Array(size);
        }
        this.colors.fill(0);
        this.depth.fill(-Infinity);
        const scale = (size * 0.46) / geometry.radius;
        const cy = Math.cos(yaw),
            sy = Math.sin(yaw),
            cp = Math.cos(pitch),
            sp = Math.sin(pitch);
        function project(point) {
            const x = point[0] - geometry.center[0],
                y = point[1] - geometry.center[1],
                z = point[2] - geometry.center[2];
            const forward = sy * x + cy * z;
            return [
                size / 2 + scale * (cy * x - sy * z),
                size / 2 - scale * (cp * y - sp * forward),
                scale * (sp * y + cp * forward)
            ];
        }
        const mappings = new Map();
        for (const face of geometry.faces) {
            const [nx, ny, nz] = face.normal;
            const forward = sy * nx + cy * nz;
            const facing = sp * ny + cp * forward;
            if (facing < 0.0001) continue;
            const dzdx = -(cy * nx - sy * nz) / facing;
            const dzdy = (cp * ny - sp * forward) / facing;
            const points = face.points.map(project);
            const yStart = Math.max(0, Math.ceil(Math.min(...points.map((p) => p[1])) - 0.5));
            const yEnd = Math.min(size, Math.ceil(Math.max(...points.map((p) => p[1])) - 0.5));
            if (yStart >= yEnd) continue;
            this.left.fill(Infinity, yStart, yEnd);
            this.right.fill(-Infinity, yStart, yEnd);
            let previous = points.at(-1);
            for (const next of points) {
                const [top, bottom] = previous[1] < next[1] ? [previous, next] : [next, previous];
                previous = next;
                const start = Math.max(yStart, Math.ceil(top[1] - 0.5));
                const end = Math.min(yEnd, Math.ceil(bottom[1] - 0.5));
                if (start >= end) continue;
                const slope = (bottom[0] - top[0]) / (bottom[1] - top[1]);
                let x = top[0] + (start + 0.5 - top[1]) * slope;
                for (let y = start; y < end; y++, x += slope) {
                    if (x < this.left[y]) this.left[y] = x;
                    if (x > this.right[y]) this.right[y] = x;
                }
            }
            const right = cy * nx - sy * nz,
                up = cp * ny - sp * forward;
            const light = 0.57 + 0.43 * Math.max(0, -0.42 * right + 0.73 * up + 0.54 * facing);
            const reflection = Math.max(0, -0.239 * right + 0.417 * up + 0.877 * facing);
            const metal = face.color[0] > face.color[2] * 1.45;
            const highlight = (metal ? 76 : 17) * Math.pow(reflection, metal ? 22 : 36);
            const color = litColor(face.color, light, highlight);
            let map;
            if (face.texture) {
                map = mappings.get(face.texture);
                if (!map) {
                    map = textureMapping(face.texture, project, texture);
                    mappings.set(face.texture, map);
                }
            }
            const [px, py, pz] = points[0];
            for (let y = yStart; y < yEnd; y++) {
                const start = Math.max(0, Math.ceil(this.left[y] - 0.5));
                const end = Math.min(size, Math.ceil(this.right[y] - 0.5));
                let z = pz + dzdx * (start + 0.5 - px) + dzdy * (y + 0.5 - py);
                let index = y * size + start;
                if (map) {
                    const dx = start + 0.5 - map.origin[0],
                        dy = y + 0.5 - map.origin[1];
                    let u = map.u + dx * map.ux + dy * map.uy;
                    let v = map.v + dx * map.vx + dy * map.vy;
                    for (let x = start; x < end; x++, index++, z += dzdx, u += map.ux, v += map.vx) {
                        if (z <= this.depth[index]) continue;
                        this.colors[index] = texel(map.pixels, u, v, color);
                        this.depth[index] = z;
                    }
                } else {
                    for (let x = start; x < end; x++, index++, z += dzdx) {
                        if (z <= this.depth[index]) continue;
                        this.colors[index] = color;
                        this.depth[index] = z;
                    }
                }
            }
        }
        this.context.putImageData(
            this.frame,
            Math.floor((width - size) / 2),
            Math.floor((height - size) / 2)
        );
    }
}
