import { mkdir, writeFile } from 'node:fs/promises';

const STYLE_KEYS = [
  'display', 'position', 'inset', 'top', 'right', 'bottom', 'left', 'zIndex',
  'flexDirection', 'flexWrap', 'alignItems', 'alignSelf', 'justifyContent', 'gap',
  'gridTemplateColumns', 'gridTemplateRows', 'width', 'height', 'minWidth', 'minHeight',
  'maxWidth', 'maxHeight', 'margin', 'padding', 'border', 'borderRadius', 'outline',
  'color', 'background', 'backgroundColor', 'backgroundImage', 'boxShadow', 'opacity',
  'fontFamily', 'fontSize', 'fontWeight', 'fontStyle', 'lineHeight', 'letterSpacing',
  'textAlign', 'textDecoration', 'textTransform', 'whiteSpace', 'overflow', 'overflowX',
  'overflowY', 'transform', 'transition', 'animationName', 'animationDuration',
  'animationTimingFunction', 'animationDelay', 'animationFillMode', 'animationPlayState',
];

function browserExtractor(selectors) {
  const styleOf = (node) => {
    const computed = getComputedStyle(node);
    return Object.fromEntries(STYLE_KEYS.map((key) => [key, computed[key]]));
  };
  const rectOf = (node) => {
    const rect = node.getBoundingClientRect();
    return {
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
      right: innerWidth - rect.right,
      bottom: innerHeight - rect.bottom,
    };
  };
  const animationOf = (node) => node.getAnimations({ subtree: true }).map((animation) => ({
    state: animation.playState,
    duration: animation.effect?.getTiming().duration ?? null,
    delay: animation.effect?.getTiming().delay ?? null,
    easing: animation.effect?.getTiming().easing ?? null,
    fill: animation.effect?.getTiming().fill ?? null,
    keyframes: animation.effect?.getKeyframes?.() ?? [],
  }));
  return {
    viewport: { width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
    document: {
      title: document.title,
      url: location.href,
      scrollWidth: document.documentElement.scrollWidth,
      scrollHeight: document.documentElement.scrollHeight,
      bodyBackground: getComputedStyle(document.body).backgroundColor,
      bodyFont: getComputedStyle(document.body).fontFamily,
    },
    selectors: Object.fromEntries(Object.entries(selectors).map(([name, selector]) => {
      const matches = [...document.querySelectorAll(selector)];
      const node = matches[0];
      if (!node) return [name, { found: false, matchCount: matches.length }];
      const svg = node.matches('svg') ? node : node.querySelector('svg');
      return [name, {
        found: true,
        matchCount: matches.length,
        tag: node.tagName,
        text: node.textContent?.replace(/\s+/g, ' ').trim() ?? '',
        attributes: Object.fromEntries([...node.attributes]
          .filter((attribute) => !attribute.name.startsWith('data-') && attribute.name !== 'class')
          .map((attribute) => [attribute.name, attribute.value])),
        rect: rectOf(node),
        style: styleOf(node),
        children: [...node.children].map((child) => ({
          tag: child.tagName,
          text: child.textContent?.replace(/\s+/g, ' ').trim().slice(0, 160) ?? '',
          rect: rectOf(child),
        })),
        svg: svg ? {
          markup: {
            attributes: Object.fromEntries([...svg.attributes]
              .map((attribute) => [attribute.name, attribute.value])
              .sort(([left], [right]) => left.localeCompare(right))),
            innerHTML: svg.innerHTML.replace(/<!---->/g, ''),
          },
          stroke: svg.getAttribute('stroke'),
          width: svg.getAttribute('width'),
          height: svg.getAttribute('height'),
          rect: rectOf(svg),
        } : null,
        animations: animationOf(node),
      }];
    })),
  };
}

const VISUAL_STYLE_KEYS = [
  'display', 'position', 'flexDirection', 'flexWrap', 'alignItems', 'alignSelf',
  'justifyContent', 'gap', 'color', 'background', 'backgroundColor', 'backgroundImage',
  'border', 'borderRadius', 'outline', 'boxShadow', 'opacity', 'fontFamily', 'fontSize',
  'fontWeight', 'fontStyle', 'lineHeight', 'letterSpacing', 'textAlign', 'textDecoration',
  'textTransform', 'whiteSpace', 'overflow', 'overflowX', 'overflowY', 'transform',
  'animationDuration', 'animationTimingFunction', 'animationDelay', 'animationFillMode',
  'animationPlayState',
];

function isObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function flatten(value, prefix = '', output = {}) {
  if (Array.isArray(value)) {
    value.forEach((item, index) => flatten(item, `${prefix}[${index}]`, output));
  } else if (isObject(value)) {
    for (const [key, item] of Object.entries(value)) {
      flatten(item, prefix ? `${prefix}.${key}` : key, output);
    }
  } else {
    output[prefix] = value;
  }
  return output;
}

export async function captureSurface(tab, selectors, options = {}) {
  const settleMs = options.settleMs ?? 600;
  if (settleMs > 0) {
    await new Promise((resolve) => setTimeout(resolve, settleMs));
  }
  const extractor = browserExtractor.toString().replace(
    'function browserExtractor(selectors) {',
    `function extractSurface() { const selectors = ${JSON.stringify(selectors)}; const STYLE_KEYS = ${JSON.stringify(STYLE_KEYS)};`,
  );
  const metrics = await tab.automation.evaluateScript(extractor);
  const screenshot = options.screenshot === false
    ? null
    : await tab.screenshot({ format: options.format ?? 'png', fullPage: options.fullPage ?? false });
  return { ...metrics, screenshot };
}

export function diffSurfaces(reference, nudge, options = {}) {
  const tolerance = options.tolerance ?? 0;
  const ignored = options.ignore ?? [];
  const { screenshot: referenceScreenshot, ...referenceMetrics } = reference;
  const { screenshot: nudgeScreenshot, ...nudgeMetrics } = nudge;
  const referenceFlat = flatten(referenceMetrics);
  const nudgeFlat = flatten(nudgeMetrics);
  const keys = [...new Set([...Object.keys(referenceFlat), ...Object.keys(nudgeFlat)])].sort();
  const differences = [];
  for (const key of keys) {
    if (options.visualOnly) {
      if (/\.(children\[|attributes\.|text$|tag$|svg\.markup)/.test(key)) {
        continue;
      }
      const styleKey = key.match(/\.style\.([^.]+)$/)?.[1];
      if (styleKey && !VISUAL_STYLE_KEYS.includes(styleKey)) {
        continue;
      }
    }
    if (ignored.some((path) => key === path || key.startsWith(`${path}.`) || key.startsWith(`${path}[`))) {
      continue;
    }
    if (key.endsWith('.screenshot') || key === 'document.url' || key === 'document.title') {
      continue;
    }
    const expected = referenceFlat[key];
    const actual = nudgeFlat[key];
    if (typeof expected === 'number' && typeof actual === 'number') {
      if (Math.abs(expected - actual) > tolerance) {
        differences.push({ path: key, reference: expected, nudge: actual, delta: actual - expected });
      }
    } else if (expected !== actual) {
      differences.push({ path: key, reference: expected, nudge: actual });
    }
  }
  return {
    matched: differences.length === 0,
    tolerance,
    differenceCount: differences.length,
    differences,
  };
}

export async function saveCapture(capture, name, directory = 'parity/ui') {
  await mkdir(directory, { recursive: true });
  const { screenshot, ...metrics } = capture;
  const jsonPath = `${directory}/${name}.json`;
  await writeFile(jsonPath, JSON.stringify(metrics, null, 2));
  let screenshotPath = null;
  if (screenshot) {
    screenshotPath = `${directory}/${name}.png`;
    await writeFile(screenshotPath, screenshot);
  }
  return { jsonPath, screenshotPath };
}

export function formatDiff(diff) {
  if (diff.matched) return 'UI parity diff: 0 differences';
  const lines = diff.differences.map(({ path, reference, nudge, delta }) => {
    const suffix = typeof delta === 'number' ? ` (delta ${delta})` : '';
    return `${path}: reference=${JSON.stringify(reference)} nudge=${JSON.stringify(nudge)}${suffix}`;
  });
  return [`UI parity diff: ${diff.differenceCount} differences`, ...lines].join('\n');
}
