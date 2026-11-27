import assert from 'node:assert/strict';
import test from 'node:test';

import { diffSurfaces } from './ui-parity-harness.mjs';

function capture(overrides = {}) {
  return {
    capture: { selectors: { box: '.box' }, settleMs: 0 },
    document: { title: 'Reference', url: 'http://reference.test' },
    selectors: {
      box: {
        attributes: { class: 'reference-class', 'data-state': 'closed' },
        rect: { x: 0, width: 100 },
        style: { color: 'rgb(0, 0, 0)', display: 'flex' },
        tag: 'DIV',
        text: 'Reference',
      },
    },
    ...overrides,
  };
}

test('default diff includes classes data attributes text and document title', () => {
  const reference = capture();
  const nudge = capture({
    document: { title: 'Nudge', url: 'http://nudge.test' },
    selectors: {
      box: {
        attributes: { class: 'nudge-class', 'data-state': 'open' },
        rect: { x: 0, width: 100 },
        style: { color: 'rgb(0, 0, 0)', display: 'flex' },
        tag: 'DIV',
        text: 'Nudge',
      },
    },
  });

  const diff = diffSurfaces(reference, nudge);
  assert.equal(diff.matched, false);
  for (const path of [
    'document.title',
    'document.url',
    'selectors.box.attributes.class',
    'selectors.box.attributes.data-state',
    'selectors.box.text',
  ]) {
    assert(diff.differences.some((difference) => difference.path === path), path);
  }
});

test('explicit ignores and tolerance are recorded in the evidence', () => {
  const reference = capture();
  const nudge = capture({
    document: { title: 'Nudge', url: 'http://nudge.test' },
    selectors: {
      box: {
        ...reference.selectors.box,
        attributes: { class: 'nudge-class', 'data-state': 'closed' },
        rect: { x: 0.01, width: 100 },
      },
    },
  });

  const diff = diffSurfaces(reference, nudge, {
    tolerance: 0.02,
    ignore: ['document.title', 'document.url', 'selectors.box.attributes.class'],
  });
  assert.equal(diff.matched, true);
  assert.equal(diff.tolerance, 0.02);
  assert.deepEqual(diff.ignored, [
    'document.title',
    'document.url',
    'selectors.box.attributes.class',
  ]);
  assert.deepEqual(diff.capture.reference, reference.capture);
  assert.deepEqual(diff.capture.nudge, nudge.capture);
});
