import { a230Display } from './a230.js';
import { iperlDisplay } from './iperl.js';
import { waterDial } from './water.js';
import { g350Index } from './g350.js';
import { elsterIndex } from './elster.js';
import { meterGeometry } from './geometry.js';
import { MeterRasterizer, textureLevels } from './rasterizer.js';
import { screens } from './screens.js';

const dialSizes = {
    'gmc-emmod206': [600, 750],
    'sensus-iperl': [600, 800],
    'elster-gas': [640, 360],
    'landisplusgyr-g350': [640, 480],
    'generic-water': [512, 512]
};

function volumeWheels(volume) {
    const match = /^(\d{1,5})(?:\.(\d+))?$/.exec(String(volume));
    return match ? match[1].padStart(5, '0') + (match[2] || '').padEnd(3, '0').slice(0, 3) : '————————';
}

function paintRegister(ctx, image, asset, reading, volume, id) {
    if (asset === 'gmc-emmod206') {
        a230Display(ctx, reading);
        return;
    }
    if (asset === 'sensus-iperl') {
        iperlDisplay(ctx, reading, id);
        return;
    }
    if (asset === 'generic-water') {
        waterDial(ctx, volumeWheels(volume), id, volume);
        return;
    }
    if (asset === 'landisplusgyr-g350') {
        g350Index(ctx, reading, id);
        return;
    }
    if (asset === 'elster-gas') {
        elsterIndex(ctx, volumeWheels(volume), id);
        return;
    }
    const w = image.naturalWidth,
        h = image.naturalHeight;
    ctx.drawImage(image, 0, 0);
    if (asset === 'gwf-mtkcoder') {
        const wheels = volumeWheels(volume);
        const x = w * 0.415,
            y = h * 0.52,
            dw = (w * 0.148) / 8,
            dh = h * 0.054;
        ctx.font = 'bold ' + dh * 0.86 + 'px monospace';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        for (let i = 0; i < 8; i++) {
            ctx.fillStyle = i < 5 ? '#242524' : '#92292a';
            ctx.fillRect(x + i * dw, y, dw + 0.2, dh);
            ctx.fillStyle = '#0d0e0c';
            ctx.fillRect(x + i * dw, y, 1, dh);
            ctx.fillStyle = '#fff';
            ctx.fillText(wheels[i], x + (i + 0.5) * dw, y + 0.52 * dh, dw * 0.95);
        }
    }
    if (asset === 'multical-601' && id !== '06855817') {
        ctx.fillStyle = '#e7e5dc';
        ctx.fillRect(w * 0.72, h * 0.18, w * 0.223, h * 0.072);
        ctx.fillRect(w * 0.77, h * 0.299, w * 0.18, h * 0.045);
        ctx.textAlign = 'left';
        ctx.textBaseline = 'middle';
        ctx.fillStyle = '#282824';
        ctx.font = 'bold ' + w * 0.038 + 'px monospace';
        ctx.fillText(id || '—', w * 0.738, h * 0.217, w * 0.19);
        ctx.font = w * 0.017 + 'px monospace';
        ctx.fillText(id || '—', w * 0.776, h * 0.323, w * 0.17);
    }
    if (!screens[asset]) return;
    const [x, y, sw, sh] = screens[asset].map((n, i) => (n * (i % 2 ? h : w)) / 100);
    const dateTime = /^(\d{4}-\d{2}-\d{2})T(.+)$/.exec(reading.value);
    const main = dateTime ? dateTime[1] : reading.value;
    const secondary = dateTime ? dateTime[2] : reading.unit;
    ctx.fillStyle = '#a9b29a';
    ctx.fillRect(x, y, sw, sh);
    ctx.fillStyle = '#263225';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.font = '500 ' + sh * 0.56 + 'px monospace';
    ctx.fillText(main, x + sw / 2, y + sh * (secondary ? 0.37 : 0.5), sw * 0.93);
    ctx.font = sh * 0.26 + 'px monospace';
    ctx.fillText(secondary, x + sw / 2, y + sh * 0.8, sw * 0.9);
}

function imageBounds({ data, width, height }) {
    let left = width,
        right = 0,
        top = height,
        bottom = 0;
    for (let y = 0; y < height; y++) {
        for (let x = 0; x < width; x++) {
            if (data[(y * width + x) * 4 + 3] < 150) continue;
            left = Math.min(left, x);
            right = Math.max(right, x);
            top = Math.min(top, y);
            bottom = Math.max(bottom, y);
        }
    }
    return right > left && bottom > top
        ? [left / width, top / height, (right - left + 1) / width, (bottom - top + 1) / height]
        : [0, 0, 1, 1];
}

class MeterTurntable extends HTMLElement {
    constructor() {
        super();
        this.canvas = document.createElement('canvas');
        this.image = document.createElement('img');
        this.image.className = 'device-image';
        this.image.alt = '';
        this.image.draggable = false;
        this.controls = document.createElement('div');
        this.controls.className = 'product-angles';
        this.controls.hidden = true;
        this.surface = document.createElement('canvas');
        this.resize = new ResizeObserver(() => this.redraw());
        this.yaw = -0.3;
        this.pitch = 0.22;
        this.addEventListener('pointerdown', (e) => {
            if (!e.isPrimary || e.button !== 0) return;
            this.drag = { id: e.pointerId, x: e.clientX, y: e.clientY, yaw: this.yaw, pitch: this.pitch };
            this.setPointerCapture(e.pointerId);
        });
        this.addEventListener('pointermove', (e) => {
            if (this.drag?.id === e.pointerId)
                this.rotate(
                    this.drag.yaw - (e.clientX - this.drag.x) * 0.013,
                    this.drag.pitch + (e.clientY - this.drag.y) * 0.013
                );
        });
        for (const type of ['pointerup', 'pointercancel', 'lostpointercapture']) {
            this.addEventListener(type, (e) => {
                if (this.drag?.id === e.pointerId) this.drag = null;
            });
        }
        this.addEventListener('keydown', (e) => {
            const step = {
                ArrowLeft: [0.18, 0],
                ArrowRight: [-0.18, 0],
                ArrowUp: [0, 0.14],
                ArrowDown: [0, -0.14],
                Home: [0, 0]
            }[e.key];
            if (!step) return;
            e.preventDefault();
            if (e.key === 'Home') this.rotate(this.initialYaw, this.initialPitch);
            else this.rotate(this.yaw + step[0], this.pitch + step[1]);
        });
    }

    connectedCallback() {
        this.resize.observe(this);
        if (this.image.complete && this.image.naturalWidth) this.loadGeometry();
    }
    disconnectedCallback() {
        this.resize.disconnect();
        cancelAnimationFrame(this.frame);
        this.frame = 0;
        this.geometry = this.texture = this.renderer = null;
        this.canvas.width = this.surface.width = 0;
        this.image.hidden = false;
        this.controls.hidden = true;
    }

    configure(asset, name, meter, records) {
        this.asset = asset;
        this.meterId = meter?.id;
        this.initialYaw = asset === 'elster-gas' ? -0.53 : -0.3;
        this.yaw = this.initialYaw;
        this.initialPitch =
            asset === 'generic-water' || asset === 'sensus-iperl'
                ? 0.97
                : asset === 'gwf-mtkcoder' || asset === 'itron-bm-plusm'
                  ? 0.75
                  : 0.22;
        this.pitch = this.initialPitch;
        this.views = [
            ['Angle', this.initialYaw, this.initialPitch],
            ['Front', 0, 0],
            ['Left', -Math.PI / 2, 0],
            ['Right', Math.PI / 2, 0],
            ['Back', Math.PI, 0],
            ['Top', 0, Math.PI / 2]
        ];
        this.controls.setAttribute('role', 'group');
        this.controls.setAttribute('aria-label', 'Camera angle');
        for (const [label, yaw, pitch] of this.views) {
            const button = document.createElement('button');
            button.type = 'button';
            button.textContent = label;
            button.setAttribute('aria-pressed', String(label === 'Angle'));
            button.addEventListener('click', () => this.rotate(yaw, pitch));
            this.controls.append(button);
        }
        this.image.alt = name;
        this.volume = records.find(
            (r) =>
                r.unit === 'm3' &&
                r.quantities.includes('Volume') &&
                !r.storage_number &&
                !r.tariff &&
                !r.subunit &&
                r.function === 'Instantaneous value'
        )?.value.value;
        this.className = 'product-stage';
        this.tabIndex = 0;
        this.setAttribute('role', 'group');
        this.setAttribute(
            'aria-label',
            name + '. Drag or use the arrow keys to rotate. Home resets the view.'
        );
        this.canvas.setAttribute('aria-hidden', 'true');
        this.append(this.image, this.canvas);
        this.image.onload = () => this.loadGeometry();
        this.image.src = new URL('./' + asset + '.webp', import.meta.url).href;
    }

    loadGeometry() {
        if (!this.isConnected || this.geometry) return;
        [this.surface.width, this.surface.height] = dialSizes[this.asset] || [
            this.image.naturalWidth,
            this.image.naturalHeight
        ];
        const ctx = this.surface.getContext('2d', { willReadFrequently: true });
        if (!ctx || !this.canvas.getContext('2d')) return;
        ctx.drawImage(this.image, 0, 0);
        const image = ctx.getImageData(0, 0, this.surface.width, this.surface.height);
        this.geometry = meterGeometry(this.asset, image, imageBounds(image));
        this.image.hidden = true;
        this.controls.hidden = false;
        this.updateReading(this.reading);
    }

    updateReading(reading = { value: '—', unit: '' }) {
        this.reading = reading;
        if (!this.geometry) return;
        paintRegister(
            this.surface.getContext('2d'),
            this.image,
            this.asset,
            reading,
            this.volume,
            this.meterId
        );
        this.texture = textureLevels(this.surface);
        this.redraw();
    }

    rotate(yaw, pitch) {
        this.yaw = yaw % (Math.PI * 2);
        this.pitch = Math.max(-Math.PI / 2, Math.min(Math.PI / 2, pitch));
        for (const [i, [, u, v]] of this.views.entries()) {
            const selected = Math.abs(Math.cos(yaw - u) - 1) < 1e-8 && Math.abs(this.pitch - v) < 1e-5;
            const button = this.controls.children[i];
            if ((button.getAttribute('aria-pressed') === 'true') !== selected)
                button.setAttribute('aria-pressed', String(selected));
        }
        this.redraw();
    }

    redraw() {
        if (!this.geometry || this.frame || !this.isConnected) return;
        this.frame = requestAnimationFrame(() => {
            this.frame = 0;
            this.draw();
        });
    }

    draw() {
        const ratio = Math.min(Math.max(window.devicePixelRatio || 1, 1.5), 2);
        const width = Math.round(this.clientWidth * ratio),
            height = Math.round(this.clientHeight * ratio);
        if (!width || !height || !this.texture) return;
        this.renderer ??= new MeterRasterizer(this.canvas);
        this.renderer.draw(this.geometry, this.texture, width, height, this.yaw, this.pitch);
    }
}

customElements.define('meter-turntable', MeterTurntable);
