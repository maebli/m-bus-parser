const assert = require('node:assert/strict');
const {readFileSync} = require('node:fs');
const {test} = require('node:test');
const {runInNewContext} = require('node:vm');
const {resolve} = require('node:path');

const html = readFileSync(resolve(__dirname, '../../docs/dev/bench/index.html'), 'utf8');
const script = html.match(/<script id="main-script">([\s\S]*?)<\/script>/)[1];

function render(entries) {
  const charts = [];
  const opened = [];
  const elements = [];
  const element = tag => {
    const node = {tag, style: {}, children: [], appendChild(child) { this.children.push(child); child.parent = this; }};
    elements.push(node);
    return node;
  };
  const window = {
    BENCHMARK_DATA: {lastUpdate: 100, repoUrl: 'https://example.test', entries},
    BENCHMARK_TAGS: {},
    open: url => opened.push(url),
  };
  runInNewContext(script, {
    window,
    document: {getElementById: element, createElement: element},
    Chart: function (canvas, config) { charts.push(config); },
  });
  return {charts, opened, elements};
}

function entry(id, date, benches) {
  return {
    commit: {id, url: `https://example.test/${id}`, message: id, timestamp: 'today', committer: {username: 'tester'}},
    date, tool: 'customSmallerIsBetter', benches,
  };
}
const bench = (name, value) => ({name, value, unit: 'bytes', extra: name});

test('legacy metrics keep their independent charts', () => {
  const {charts} = render({Resources: [entry('a', 1, [bench('Stack', 12), bench('Flash', 80)])]});
  assert.equal(charts.length, 2);
  assert.equal(charts[0].data.datasets.length, 1);
  assert.equal(charts[0].data.datasets[0].label, 'Stack');
  assert.equal(charts[0].data.datasets[0].data[0], 12);
});

test('optimization profiles share a chart and align missing values and reruns', () => {
  const {charts, opened} = render({Comparison: [
    entry('a', 1, [bench('Flash [opt-level=z]', 20)]),
    entry('b', 2, [bench('Flash [opt-level=z]', 21), bench('Flash [opt-level=s]', 22), bench('Flash [opt-level=3]', 30)]),
    entry('b', 3, [bench('Flash [opt-level=z]', 23), bench('Flash [opt-level=3]', 31)]),
  ]});
  assert.equal(charts.length, 1);
  const chart = charts[0];
  assert.equal(chart.options.title.text, 'Flash');
  assert.deepEqual(Array.from(chart.data.datasets, d => Array.from(d.data)), [[20, 21, 23], [null, 22, null], [null, 30, 31]]);
  assert.equal(new Set(chart.data.datasets.map(d => d.borderColor)).size, 3);
  const tooltip = {datasetIndex: 2, index: 1, value: '30'};
  assert.match(chart.options.tooltips.callbacks.label(tooltip), /Speed \(3\): 30 bytes/);
  assert.match(chart.options.tooltips.callbacks.afterLabel(tooltip), /opt-level=3/);
  assert.match(chart.options.tooltips.callbacks.afterTitle([tooltip]), /b/);
  chart.options.onClick(null, [{_datasetIndex: 2, _index: 1}]);
  assert.deepEqual(opened, ['https://example.test/b']);
});


test('single-run zero-valued profiles remain visible with separate chart containers', () => {
  const {charts, elements} = render({
    'Parser stack and footprint': [entry('old', 1, [bench('Stack', 12)])],
    'Parser optimization comparison': [entry('new', 2, [
      bench('RAM [opt-level=3]', 0), bench('RAM [opt-level=s]', 0), bench('RAM [opt-level=z]', 0),
    ])],
  });
  assert.equal(charts[0].options.title.text, 'RAM');
  assert.deepEqual(Array.from(charts[0].data.datasets, d => d.label), ['Size (z)', 'Size (s)', 'Speed (3)']);
  const canvases = elements.filter(e => e.tag === 'canvas');
  assert.equal(new Set(canvases.map(e => e.parent)).size, 2);
  for (const canvas of canvases) {
    assert.equal(canvas.parent.className, 'benchmark-plot');
    assert.equal(canvas.parent.children.length, 1);
  }
  assert.ok(charts.every(c => c.options.maintainAspectRatio === false));
  assert.equal(elements.filter(e => e.textContent === '0 bytes').length, 3);
  assert.equal(elements.filter(e => e.className === 'benchmark-note' && e.textContent.includes('1 recorded run')).length, 2);
});

test('Rust/libmbus comparisons overlay only equivalent workloads and retain missing runs', () => {
  const {charts, opened} = render({'Parser corpus and libmbus': [
    entry('a', 1, [bench('Wired link-layer comparison [implementation=rust]', 20)]),
    entry('b', 2, [
      bench('Wired link-layer comparison [implementation=libmbus]', 45),
      bench('Wired link-layer comparison [implementation=rust]', 21),
      bench('Wired equivalent XML comparison [implementation=rust]', 300),
      bench('Wired equivalent XML comparison [implementation=libmbus]', 600),
    ]),
  ]});
  assert.equal(charts.length, 2);
  assert.deepEqual(Array.from(charts[0].data.datasets, d => d.label), ['Rust (O3)', 'libmbus (O3)']);
  assert.deepEqual(Array.from(charts[0].data.datasets, d => Array.from(d.data)), [[20, 21], [null, 45]]);
  assert.equal(charts[1].options.title.text, 'Wired equivalent XML comparison');
  const item = {datasetIndex: 1, index: 1, value: '45'};
  assert.match(charts[0].options.tooltips.callbacks.label(item), /libmbus \(O3\): 45/);
  charts[0].options.onClick(null, [{_datasetIndex: 1, _index: 1}]);
  assert.deepEqual(opened, ['https://example.test/b']);
});

test('CRC copy/view share charts per payload length without merging profiles or sizes', () => {
  const {charts} = render({'Parser corpus and libmbus': [entry('a', 1, [
    bench('Format A normalization (16 payload bytes) [method=view]', 100),
    bench('Format A normalization (16 payload bytes) [method=copy]', 80),
    bench('Format A normalization (32 payload bytes) [method=copy]', 150),
    bench('Wired corpus full semantic decode [opt-level=z]', 900),
  ])]});
  assert.equal(charts.length, 3);
  assert.deepEqual(Array.from(charts[0].data.datasets, d => d.label), ['Copy to buffer', 'Borrowed view']);
  assert.deepEqual(Array.from(charts[0].data.datasets, d => Array.from(d.data)), [[80], [100]]);
  assert.equal(charts[2].data.datasets[0].label, 'Size (z)');
});

test('exported summary displays one comparison table with three rows', {
  skip: !process.env.BENCH_CORPUS_JSON,
}, () => {
  const metrics = JSON.parse(readFileSync(process.env.BENCH_CORPUS_JSON, 'utf8'));
  assert.equal(metrics.length, 6);
  const {charts, elements} = render({'Parser library comparison': [entry('measured', 1, metrics)]});
  assert.equal(charts.length, 3); // optional historical charts remain in a closed details element
  assert.equal(elements.filter(e => e.className === 'comparison-table').length, 1);
  assert.equal(elements.filter(e => e.className === 'comparison-value').length, 6);
  const comparison = elements.find(e => e.className === 'comparison-table');
  assert.equal(comparison.children.length, 4); // header + speed, stack, flash
  assert.ok(elements.some(e => e.textContent && e.textContent.includes('same 73 wired meter messages')));
  assert.ok(elements.some(e => e.textContent && e.textContent.includes('additional working memory')));
  assert.ok(html.includes('<details id="history-details">'));
});

test('the overview never mixes old memory results into a newer partial run', () => {
  const {elements} = render({'Parser library comparison': [
    entry('old', 1, [bench('Peak decoder stack [implementation=libmbus]', 500)]),
    entry('new', 2, [{name: 'Corpus decode latency [implementation=rust]', unit: 'ns/frame', value: 1000}]),
  ]});
  const values = elements.filter(e => e.className === 'comparison-value');
  assert.equal(values.filter(e => e.textContent === 'Not measured').length, 5);
});

test('instruction counts stay separate from timing and retain their units', () => {
  const {charts} = render({'Parser instruction counts': [entry('a', 1,
    ['rust', 'libmbus'].map(implementation => ({
      name: `Decoder instructions [implementation=${implementation}]`,
      value: 1200, unit: 'instructions/message',
    })))], 'Parser library comparison': [entry('a', 1, [
      {name: 'Corpus decode latency [implementation=rust]', value: 500, unit: 'ns/frame'},
    ])]});
  assert.equal(charts.length, 2);
  assert.equal(charts[1].options.title.text, 'Decoder instructions');
  assert.equal(charts[1].data.datasets.length, 2);
  assert.match(charts[1].options.tooltips.callbacks.label({datasetIndex: 0, index: 0, value: '1200'}), /instructions\/message/);
});
