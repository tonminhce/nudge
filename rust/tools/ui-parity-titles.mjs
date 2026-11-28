import { readFileSync } from 'node:fs';

import {
  captureSurface,
  diffSurfaces,
  formatDiff,
  openSession,
  saveCapture,
  saveDiff,
} from './ui-parity-cdp.mjs';

const STATE_PATH = process.env.NUDGE_TITLE_STATE ?? '/tmp/nudge-locale-parity-state.json';
const REFERENCE_BASE = process.env.NUDGE_REFERENCE_BASE ?? 'http://locale-f315f4672f6e.localhost:3001';
const REFERENCE_API = process.env.NUDGE_REFERENCE_API ?? 'http://127.0.0.1:3001';
const NUDGE_BASE = process.env.NUDGE_FRONT_BASE ?? 'http://127.0.0.1:8083';
const NUDGE_API = process.env.NUDGE_SERVER_BASE ?? 'http://127.0.0.1:3018';
const RECORD_ID = process.env.NUDGE_RECORD_ID ?? '56e27a4a-e9f1-4376-9d62-3b8909ebb8af';

const HEAD_SELECTORS = {
  favicon: 'link[rel="icon"]',
  appleTouchIcon: 'link[rel="apple-touch-icon"]',
};

const HEAD_IGNORE = [
  'viewport',
  'document.url',
  'document.scrollWidth',
  'document.scrollHeight',
  'document.bodyBackground',
  'document.bodyFont',
  'selectors.favicon.rect',
  'selectors.favicon.style',
  'selectors.favicon.children',
  'selectors.favicon.animations',
  'selectors.favicon.svg',
  'selectors.favicon.text',
  'selectors.favicon.tag',
  'selectors.appleTouchIcon.rect',
  'selectors.appleTouchIcon.style',
  'selectors.appleTouchIcon.children',
  'selectors.appleTouchIcon.animations',
  'selectors.appleTouchIcon.svg',
  'selectors.appleTouchIcon.text',
  'selectors.appleTouchIcon.tag',
];

async function gql(base, path, query, variables) {
  const response = await fetch(base + path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ query, variables: variables ?? {} }),
  });
  return response.json();
}

async function referenceLoginToken(state) {
  const body = await gql(
    REFERENCE_API,
    '/metadata',
    `mutation signIn($email: String!, $password: String!) { signIn(email: $email, password: $password) { availableWorkspaces { availableWorkspacesForSignIn { id loginToken } } } }`,
    { email: state.email, password: state.password },
  );
  const workspaces = body?.data?.signIn?.availableWorkspaces?.availableWorkspacesForSignIn ?? [];
  if (!workspaces.length) throw new Error('reference signIn returned no workspace');
  return workspaces[0].loginToken;
}

async function nudgeToken(state) {
  const signedIn = await gql(
    NUDGE_API,
    '/metadata',
    `mutation signIn($email: String!, $password: String!) { signIn(email: $email, password: $password) { availableWorkspaces { availableWorkspacesForSignIn { id loginToken } } } }`,
    { email: state.email, password: state.password },
  );
  const workspaces =
    signedIn?.data?.signIn?.availableWorkspaces?.availableWorkspacesForSignIn ?? [];
  if (!workspaces.length) throw new Error('nudge signIn returned no workspace');
  const exchanged = await gql(
    NUDGE_API,
    '/metadata',
    `mutation ex($loginToken: String!, $origin: String!) { getAuthTokensFromLoginToken(loginToken: $loginToken, origin: $origin) { tokens { accessOrWorkspaceAgnosticToken { token } } } }`,
    { loginToken: workspaces[0].loginToken, origin: NUDGE_API },
  );
  const token =
    exchanged?.data?.getAuthTokensFromLoginToken?.tokens?.accessOrWorkspaceAgnosticToken?.token;
  if (!token) throw new Error('nudge token exchange failed');
  return { token, workspaceId: workspaces[0].id };
}

function surfaces() {
  return [
    { name: 'signin', path: '/welcome', authed: false },
    { name: 'password-reset', path: '/reset-password/parity-token', authed: false },
    { name: 'create-profile', path: '/create/profile', authed: false },
    { name: 'people-index', path: '/objects/people', authed: true },
    { name: 'record-show', path: `/object/person/${RECORD_ID}`, authed: true },
    { name: 'settings-general', path: '/settings/general', authed: true },
    { name: 'command-menu', path: '/objects/people', authed: true, openMenu: true },
  ];
}

async function captureApp(session, base, kind, state) {
  const results = [];
  if (kind === 'nudge') {
    await session.goto(`${base}/welcome`);
    await session.settle(1500);
    await session.storage({
      'nudge.serverUrl': NUDGE_API,
      'nudge.token': state.token,
      'nudge.workspaceId': state.workspaceId,
    });
  }
  let referenceAuthenticated = false;
  for (const surface of surfaces()) {
    let url = base + surface.path;
    if (kind === 'reference' && surface.authed) {
      if (!referenceAuthenticated) {
        const loginToken = await referenceLoginToken(state);
        url = `${base}/objects/people?loginToken=${loginToken}`;
        await session.goto(url);
        await session.settle(6000);
        const marker = await session.evaluate('document.title');
        referenceAuthenticated = marker !== 'Sign in or Create an account';
        url = base + surface.path;
      }
    }
    await session.goto(url);
    await session.settle(surface.authed ? 4500 : 2500);
    if (surface.openMenu) {
      await session.evaluate(
        `document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true, bubbles: true, cancelable: true })); 'dispatched'`,
      );
      await session.settle(1500);
    }
    const capture = await captureSurface(session.tab, HEAD_SELECTORS, {
      settleMs: 400,
      screenshot: false,
    });
    results.push({
      name: surface.name,
      title: capture.document.title,
      iconHref: capture.selectors.favicon.attributes?.href ?? null,
      iconType: capture.selectors.favicon.attributes?.type ?? null,
      iconFound: capture.selectors.favicon.found,
      iconCount: capture.selectors.favicon.matchCount,
      appleHref: capture.selectors.appleTouchIcon.attributes?.href ?? null,
      menuOpen: surface.openMenu
        ? await session.evaluate('Boolean(document.querySelector(".command-menu-panel, [role=\\"dialog\\"]"))')
        : null,
      capture,
    });
  }
  return results;
}

const state = JSON.parse(readFileSync(STATE_PATH, 'utf8'));
const session = await openSession({ port: 9345 });
const nudgeAuth = await nudgeToken(state);
const reference = await captureApp(session, REFERENCE_BASE, 'reference', state);
const nudge = await captureApp(session, NUDGE_BASE, 'nudge', { ...state, ...nudgeAuth });

const rows = [];
for (let index = 0; index < reference.length; index += 1) {
  const left = reference[index];
  const right = nudge[index];
  const diff = diffSurfaces(left.capture, right.capture, { ignore: HEAD_IGNORE });
  const savedDiff = await saveDiff(diff, `head-${left.name}-diff`);
  await saveCapture({ ...left.capture, screenshot: null }, `head-${left.name}-reference`);
  await saveCapture({ ...right.capture, screenshot: null }, `head-${right.name}-nudge`);
  rows.push({
    surface: left.name,
    referenceTitle: left.title,
    nudgeTitle: right.title,
    titleMatch: left.title === right.title,
    referenceIcon: left.iconHref,
    nudgeIcon: right.iconHref,
    iconTypeMatch: left.iconType === right.iconType,
    headDifferences: diff.differences.map((entry) => `${entry.path} ref=${JSON.stringify(entry.reference)} nudge=${JSON.stringify(entry.nudge)}`),
    diffPath: savedDiff.jsonPath,
  });
}

for (const row of rows) {
  console.log(`\n== ${row.surface} ==`);
  console.log(`title  reference=${JSON.stringify(row.referenceTitle)} nudge=${JSON.stringify(row.nudgeTitle)} match=${row.titleMatch}`);
  console.log(`icon   reference=${JSON.stringify(row.referenceIcon)} nudge=${JSON.stringify(row.nudgeIcon)} typeMatch=${row.iconTypeMatch}`);
  if (row.headDifferences.length) console.log(`head   ${row.headDifferences.join(' | ')}`);
  console.log(`diff   ${row.diffPath} ${formatDiff({ matched: row.headDifferences.length === 0, differences: row.headDifferences, differenceCount: row.headDifferences.length })}`);
}

console.log('\nsummary ' + JSON.stringify(rows.map((row) => ({
  surface: row.surface,
  titleMatch: row.titleMatch,
  iconTypeMatch: row.iconTypeMatch,
  headDifferences: row.headDifferences.length,
})), null, 2));

await session.close();
