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
    const node = {tag, children: [], appendChild(child) { this.children.push(child); child.parent = this; }};
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
