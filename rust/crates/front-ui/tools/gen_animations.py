import json
import os
import subprocess
import sys

NODE_SCRIPT = r"""
const fs = require('node:fs');
const path = require('node:path');
const postcss = require('postcss');
const root = '/app/packages/twenty-server/dist/front/assets';
const files = fs.readdirSync(root).filter((file) => file.endsWith('.css')).sort();
const keyframes = new Map();
const rules = new Map();
const reduced = new Map();
for (const file of files) {
  const css = fs.readFileSync(path.join(root, file), 'utf8');
  const tree = postcss.parse(css, { from: file });
  tree.walkAtRules('keyframes', (rule) => {
    const value = rule.toString();
    keyframes.set(value, value);
  });
  tree.walkRules((rule) => {
    const declarations = (rule.nodes ?? []).filter((node) =>
      node.type === 'decl' && /^(animation|transition)(-|$)/.test(node.prop)
    );
    if (declarations.length === 0) return;
    const value = declarations.map((node) => node.toString()).join(';');
    let isReduced = false;
    for (let parent = rule.parent; parent; parent = parent.parent) {
      if (parent.type === 'atrule' && parent.name === 'media' && parent.params.includes('prefers-reduced-motion')) {
        isReduced = true;
        break;
      }
    }
    const target = isReduced ? reduced : rules;
    target.set(`${rule.selector}{${value}}`, `${rule.selector}{${value}}`);
  });
}
const jsFiles = fs.readdirSync(root).filter((file) => file.endsWith('.js')).sort();
const jsKeyframeNames = [
  'headerbackground',
  'swipe-out-left',
  'swipe-out-right',
  'swipe-out-up',
  'swipe-out-down',
  'sonner-fade-in',
  'sonner-fade-out',
  'sonner-spin',
  'ProseMirror-cursor-blink',
];
for (const file of jsFiles) {
  const source = fs.readFileSync(path.join(root, file), 'utf8');
  for (const name of jsKeyframeNames) {
    const marker = `@keyframes ${name}`;
    const start = source.indexOf(marker);
    if (start < 0) continue;
    const open = source.indexOf('{', start);
    let depth = 0;
    let end = open;
    for (; end < source.length; end += 1) {
      if (source[end] === '{') depth += 1;
      if (source[end] === '}') depth -= 1;
      if (depth === 0) break;
    }
    const value = source.slice(start, end + 1);
    keyframes.set(value, value);
  }
}
for (const name of ['cm-blink', 'cm-blink2']) {
  const value = `@keyframes ${name}{0%{}50%{opacity:0}100%{}}`;
  keyframes.set(value, value);
}
for (const value of [
  '[data-sonner-toast]{animation:sonner-fade-in .2s ease-out both}',
  '[data-sonner-toast][data-visible=false]{animation:sonner-fade-out .2s ease-in both}',
  '.sonner-loader{animation:sonner-spin 1s linear infinite}',
  '.cm-cursor,.cm-dropCursor{animation:cm-blink 1.2s step-end infinite}',
  '.ProseMirror-cursor{animation:ProseMirror-cursor-blink 1.2s step-end infinite}',
]) {
  rules.set(value, value);
}
process.stdout.write(JSON.stringify({
  keyframes: [...keyframes.values()],
  rules: [...rules.values()],
  reduced: [...reduced.values()],
}));
"""

UTILITIES = """
[data-animation="fade-in"] { animation: fadein calc(var(--t-animation-duration-normal) * 1s) ease-out both; }
[data-animation="fade-out"] { animation: fadeOut calc(var(--t-animation-duration-fast) * 1s) ease-in both; }
[data-animation="spin"] { animation: codicon-spin 1s linear infinite; }
[data-animation="pulse"] { animation: pulse 1.5s ease-in-out infinite; }
[data-animation="shimmer"] { animation: shimmer-wave-s1mdoo3h 1.5s ease-in-out infinite; }
[data-animation="caret"] { animation: caret-blink-s1cz26c7 1.06s step-end infinite; }
[data-animation="checkmark"] { animation: animated-checkmark-draw-checkmark-adh6qgv .8s ease-out both; }
[data-animation="empty-state"] { animation: emptyStateFadeIn-s15il85r calc(var(--t-animation-duration-normal) * 1s) ease-out both; }
[data-animation="float-in"] { animation: gettingStartedStepPreviewFloatingCardIn-s159xoc4 calc(var(--t-animation-duration-normal) * 1s) cubic-bezier(.33, 1, .68, 1) both; }
[data-animation="modal"] { animation: fadein-modal-b002945c calc(var(--t-animation-duration-normal) * 1s) ease-out both; }
[data-animation="side-panel"] { animation: sidePanelShrinkFromFullWidth-sobflnn calc(var(--t-animation-duration-normal) * 1s) ease-out both; }
[data-animation="toast-in"] { animation: sonner-fade-in .2s ease-out both; }
[data-animation="toast-out"] { animation: sonner-fade-out .2s ease-in both; }
[data-animation="swipe-left"] { animation: swipe-out-left .2s ease-out both; }
[data-animation="swipe-right"] { animation: swipe-out-right .2s ease-out both; }
[data-animation="swipe-up"] { animation: swipe-out-up .2s ease-out both; }
[data-animation="swipe-down"] { animation: swipe-out-down .2s ease-out both; }
[data-animation="editor-cursor"] { animation: ProseMirror-cursor-blink 1.2s step-end infinite; }

.t-button { transition: background-color calc(var(--t-animation-duration-instant) * 1s) ease, border-color calc(var(--t-animation-duration-instant) * 1s) ease, color calc(var(--t-animation-duration-instant) * 1s) ease; }
.t-input { transition: background-color calc(var(--t-animation-duration-normal) * 1s) ease, border-color calc(var(--t-animation-duration-normal) * 1s) ease, color calc(var(--t-animation-duration-normal) * 1s) ease; }
.app-icon-button, .view-tab, .drawer-item { transition: background-color calc(var(--t-animation-duration-fast) * 1s) ease, color calc(var(--t-animation-duration-fast) * 1s) ease, transform calc(var(--t-animation-duration-fast) * 1s) ease; }
.record-table-row { transition: background-color calc(var(--t-animation-duration-instant) * 1s) ease; }
.modal-backdrop { animation: fadein calc(var(--t-animation-duration-normal) * 1s) ease-out both; }
.create-dialog { animation: fadein-modal-b002945c calc(var(--t-animation-duration-normal) * 1s) ease-out both; }
"""

REDUCED_FALLBACK = """
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: .01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: .01ms !important;
    scroll-behavior: auto !important;
  }
}
"""


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: gen_animations.py OUTPUT_CSS")
    container = os.environ.get("TWENTY_REFERENCE_CONTAINER", "twenty-ref")
    data = json.loads(subprocess.check_output([
        "docker", "exec", container, "node", "-e", NODE_SCRIPT,
    ]))
    lines = ["@layer twenty-ui-motion {"]
    lines.extend(data["keyframes"])
    lines.extend(data["rules"])
    lines.append(UTILITIES.strip())
    lines.append("}")
    if data["reduced"]:
        lines.append("@media (prefers-reduced-motion: reduce) {")
        lines.extend(data["reduced"])
        lines.append("}")
    lines.append(REDUCED_FALLBACK.strip())
    with open(sys.argv[1], "w", encoding="utf-8") as output:
        output.write("\n".join(lines) + "\n")
    print(
        f"wrote {len(data['keyframes'])} keyframes, {len(data['rules'])} animation/transition rules "
        f"and {len(data['reduced'])} reduced-motion rules to {sys.argv[1]}"
    )


if __name__ == "__main__":
    main()
