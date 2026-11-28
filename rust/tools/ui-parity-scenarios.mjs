import { parseArgs } from 'node:util';

import { openSession } from './ui-parity-cdp.mjs';
import { captureSurface, diffSurfaces, formatDiff, saveCapture, saveDiff } from './ui-parity-harness.mjs';

const REFERENCE = 'http://127.0.0.1:3001';
const NUDGE = `http://127.0.0.1:${process.env.NUDGE_PARITY_PORT ?? 3023}`;

const BRAND_PATTERN = /nudge|twenty|my workspace/i;

const ATTR_IGNORES = ['attributes.class', 'attributes.id', 'attributes.style', 'attributes.for'];

const scenarios = {
  welcome: {
    route: '/welcome',
    ignoreSelectors: [...ATTR_IGNORES, 'selectors.*.svg.markup.innerHTML'],
    textSelectors: ['badgeFallback', 'headingText'],
    reference: {
      layout: '.swdi819',
      brand: '.f5jqen4',
      logo: 'a[href="/welcome"]',
      badgeWrap: 'div:has(> span[class^="_root_"])',
      badge: 'span[class^="_root_"]',
      badgeFallback: 'span[class^="_fallback_"]',
      headingWrap: '.sfyinng',
      headingText: '.sfyinng > div > div',
      form: 'form',
      field: '.svbhmn0',
      input: 'input[autocomplete="email"]',
      btnWrap: '.su9hunl',
      submit: 'button[type="submit"]',
      footer: 'div:has(> a[href*="privacy"])',
      privacy: 'a[href*="privacy"]',
      terms: 'a[href*="terms"]',
    },
    nudge: {
      layout: '.auth-card',
      brand: '.auth-brand',
      logo: '.auth-logo',
      badgeWrap: '.auth-workspace-badge-wrap',
      badge: '.auth-workspace-badge',
      badgeFallback: '.auth-workspace-badge > span',
      headingWrap: '.auth-heading',
      headingText: '.auth-heading-text',
      form: '.auth-form',
      field: '.auth-field',
      input: '.auth-input',
      btnWrap: '.auth-submit-row',
      submit: '.auth-submit',
      footer: '.auth-footer',
      privacy: 'a[href*="privacy"]',
      terms: 'a[href*="terms"]',
    },
  },
  signin: {
    route: '/welcome',
    prepare: (tab) => tab.focus('input[autocomplete="email"]'),
    ignoreSelectors: [...ATTR_IGNORES, 'selectors.*.svg.markup.innerHTML'],
    textSelectors: ['badgeFallback', 'headingText'],
    reference: {
      layout: '.swdi819',
      headingText: '.sfyinng > div > div',
      field: '.svbhmn0',
      input: 'input[autocomplete="email"]',
      submit: 'button[type="submit"]',
    },
    nudge: {
      layout: '.auth-card',
      headingText: '.auth-heading-text',
      field: '.auth-field',
      input: '.auth-input',
      submit: '.auth-submit',
    },
  },
  password: {
    route: '/welcome',
    prepare: async (tab) => {
      await tab.clickText('Forgot your password');
      await tab.settle(900);
    },
    ignoreSelectors: [...ATTR_IGNORES, 'selectors.*.svg.markup.innerHTML', 'selectors.*.attributes.href'],
    textSelectors: ['badgeFallback', 'headingText', 'hint'],
    reference: {
      layout: '.swdi819',
      headingText: '.sfyinng > div > div',
      form: 'form',
      email: 'input[autocomplete="email"]',
      submit: 'button[type="submit"]',
      back: 'form button:not([type="submit"])',
    },
    nudge: {
      layout: '.auth-card',
      headingText: '.auth-heading-text',
      form: '.auth-form',
      email: '.auth-input',
      submit: '.auth-submit',
      back: '.auth-forgot',
    },
  },
  toast: {
    route: '/welcome',
    prepare: async (tab) => {
      await tab.fill('input[autocomplete="email"]', 'parity@nudge.test');
      await tab.click('button[type="submit"]');
      await tab.wait('Boolean(document.querySelector("[class*=toast], [role=status], [role=alert]"))', 9000);
    },
    ignoreSelectors: [...ATTR_IGNORES, 'selectors.*.svg.markup.innerHTML', 'selectors.*.animationName'],
    textSelectors: ['message', 'heading'],
    reference: {
      toast: '[class*="toast"], [role="status"]',
      icon: '[class*="toast"] svg, [role="status"] svg',
      close: '[class*="toast"] button, [role="status"] button',
    },
    nudge: {
      toast: '[class*="toast"], [role="status"]',
      icon: '[class*="toast"] svg, [role="status"] svg',
      close: '[class*="toast"] button, [role="status"] button',
    },
  },
  'welcome-common': {
    route: '/welcome',
    visualOnly: true,
    ignoreSelectors: [...ATTR_IGNORES],
    textSelectors: [],
    reference: {
      body: 'body',
      form: 'form',
      input: 'form input',
      button: 'form button',
      privacy: 'a[href*="privacy"]',
      terms: 'a[href*="terms"]',
    },
    nudge: {
      body: 'body',
      form: 'form',
      input: 'form input',
      button: 'form button',
      privacy: 'a[href*="privacy"]',
      terms: 'a[href*="terms"]',
    },
  },
};

const outlineScript = `(() => {
  const name = (node) => node.tagName.toLowerCase()
    + (node.id ? '#' + node.id : '')
    + (node.className && typeof node.className === 'string' ? '.' + node.className.trim().split(/\\s+/).join('.') : '');
  const walk = (node, depth) => {
    if (depth > 14) return [];
    const text = [...node.childNodes].filter((c) => c.nodeType === 3).map((c) => c.textContent.trim()).join(' ');
    const rect = node.getBoundingClientRect();
    const line = '  '.repeat(depth) + name(node) + (text ? ' "' + text.slice(0, 60) + '"' : '')
      + ' ' + Math.round(rect.x) + ',' + Math.round(rect.y) + ' ' + Math.round(rect.width) + 'x' + Math.round(rect.height);
    return [line].concat([...node.children].flatMap((child) => walk(child, depth + 1)));
  };
  const root = document.querySelector('#root');
  return root ? walk(root, 0).join('\\n') : 'NO ROOT';
})();`;

function ignoresFor(scenario) {
  const keys = Object.keys(scenario.nudge);
  const names = ['document.title', 'document.url', 'document.bodyBackground', 'document.bodyFont'];
  for (const suffix of scenario.ignoreSelectors ?? []) {
    if (!suffix.includes('*')) {
      names.push(suffix);
      continue;
    }
    const tail = suffix.split('.').pop();
    for (const key of keys) names.push(`selectors.${key}.${tail}`);
  }
  for (const key of scenario.textSelectors ?? []) names.push(`selectors.${key}.text`);
  return [...new Set(names)];
}

async function runScenario(name, scenario, options) {
  const surfaces = {};
  const sessions = [];
  for (const [surface, base, selectors] of [['reference', REFERENCE, scenario.reference], ['nudge', NUDGE, scenario.nudge]]) {
    const session = await openSession({ port: options.port + (surface === 'reference' ? 0 : 1) });
    sessions.push(session);
    await session.goto(base + scenario.route);
    await session.wait('Boolean(document.querySelector("form"))', 30000);
    await session.settle(1200);
    if (scenario.prepare) {
      await scenario.prepare(session);
    }
    if (options.outline) {
      console.log(`\n===== ${name} / ${surface} outline =====`);
      console.log(await session.evaluate(outlineScript));
      continue;
    }
    const capture = await captureSurface(session.tab, selectors, { settleMs: options.settleMs ?? 900 });
    surfaces[surface] = capture;
    await saveCapture(capture, `${name}-${surface}`, options.directory);
  }
  if (options.outline) {
    for (const session of sessions) await session.close();
    return null;
  }
  const diff = diffSurfaces(surfaces.reference, surfaces.nudge, {
    tolerance: 0,
    visualOnly: scenario.visualOnly === true,
    ignore: ignoresFor(scenario),
  });
  const branding = diff.differences.filter((entry) => BRAND_PATTERN.test(`${entry.reference}${entry.nudge}`));
  await saveDiff(diff, `${name}-diff`, options.directory);
  console.log(`\n=== ${name} === differences ${diff.differenceCount}`);
  console.log(formatDiff(diff).split('\n').slice(0, 40).join('\n'));
  console.log(`branding-attributable entries: ${branding.length}`);
  for (const session of sessions) await session.close();
  return diff;
}

const { values: options } = parseArgs({
  options: {
    scenario: { type: 'string' },
    outline: { type: 'boolean', default: false },
    directory: { type: 'string', default: 'parity/ui' },
    port: { type: 'string', default: '9441' },
  },
});

const requested = options.scenario ? [options.scenario] : Object.keys(scenarios);
const totals = [];
for (const name of requested) {
  const diff = await runScenario(name, scenarios[name], { ...options, port: Number(options.port) });
  if (diff) totals.push(`${name}=${diff.differenceCount}`);
}
console.log(`\nsummary: ${totals.join(' ')}`);
