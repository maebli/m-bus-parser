import { faceSize, paintFace } from './faces.js';
import { meterGeometry } from './geometry.js';
import { MeterRasterizer, textureLevels } from './rasterizer.js';
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
        this.loadGeometry();
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
        this.loadGeometry();
    }

    loadGeometry() {
        if (!this.isConnected || !this.asset || this.geometry) return;
        if (!this.surface.getContext('2d', { willReadFrequently: true }) || !this.canvas.getContext('2d')) {
            this.image.src = new URL('./' + this.asset + '.webp', import.meta.url).href;
            return;
        }
        [this.surface.width, this.surface.height] = faceSize(this.asset);
        this.geometry = meterGeometry(this.asset);
        this.image.hidden = true;
        this.controls.hidden = false;
        this.updateReading(this.reading);
    }

    updateReading(reading = { value: '—', unit: '' }) {
        this.reading = reading;
        if (!this.geometry) return;
        paintFace(this.surface.getContext('2d'), this.asset, reading, this.volume, this.meterId);
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
