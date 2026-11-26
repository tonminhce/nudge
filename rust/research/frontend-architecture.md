# Wave 5 — Front-end architecture (Twenty → Rust/Leptos port of `twenty-front`)

Status: **research + implementable plan** (no product code in this unit). Branch
`wave5/frontend-research`; deliverable is this document.

Upstream reference: the Twenty monorepo at tag **`twenty/v2.61.0`** — the version this
port pins (`rust/MAP.md:10` "Pinned to Twenty **2.61.0**"; ADR-006/009 reference the
`twentycrm/twenty:2.61.0` image and the `rust/baseline/core-2.61.0.sql` golden dump,
`rust/DECISIONS.md:34,49-53`; container `twentycrm/twenty:v2.61.0` running as `twenty-ref`
on port 3001 per `rust/MAP.md:16-22`). Every upstream
`file:line` below is verified against that tag (`git archive twenty/v2.61.0`), **not**
against the working-tree HEAD of `/Users/tonminh-mac/Documents/GitHub/twenty`, which has
moved to **2.65.0** (`packages/twenty-server/src/engine/core-modules/upgrade/constants/twenty-current-version.constant.ts:13`
in the checkout reads `'2.65.0'`). Where the local `research/frontend-inventory.md`
(wave 0) drifted or left a claim uncited, this doc re-grounds it at 2.61.0 and flags the
correction.

Path convention: `twenty-front/...`, `twenty-ui/...`, `twenty-shared/...`,
`twenty-sdk/...` are relative to the Twenty monorepo root at `twenty/v2.61.0`. Nudge paths
are relative to `rust/` in this repo; `README.md` is the repo-root readme
(one level above `rust/`).

Committed direction (do not re-litigate; this doc implements it): the UI is a **pure-Rust
Leptos CSR app built with `trunk`, wrapped by a Tauri 2 shell** — ADR-003
(`rust/DECISIONS.md:17-20`), ADR-008 (`rust/DECISIONS.md:43-47`), and the ASCII architecture
box that labels the client "Leptos UI (wasm, CSR)" over a "Tauri 2 shell (desktop)"
(`README.md:133-144`). No reuse of the React code (`rust/DECISIONS.md:19-20`,
`rust/MAP.md:105-106`). Wave 5's scope line is "Leptos UI shell + auth + record
table/kanban/show page + settings, wrapped in Tauri" (`rust/MAP.md:33`).

---

## 1. Information-architecture inventory of `twenty-front`

### 1.1 Two routers, mounted on login state

Upstream builds **two** `react-router` v7 data routers and swaps between them on auth
state, not on a URL prefix:

- Pre-workspace (root) router — `twenty-front/src/modules/app/hooks/useCreateRootAppRouter.tsx:31-76`,
  mounted by `RootApp.tsx:4-8`. Its routes: `AppPath.SignInUp` (`/welcome`) and
  `AppPath.Invite` (`/invite/:workspaceInviteHash`) inside `BlankLayout`+`OnboardingTransitionOutlet`
  (`useCreateRootAppRouter.tsx:39-58`); `AppPath.VerifyEmail` and `AppPath.ResetPassword`
  inside `AuthFlowLayout` (`useCreateRootAppRouter.tsx:59-69`); and a catch-all
  `AppPath.NotFoundWildcard` → `RootDeepLinkRedirect` (`useCreateRootAppRouter.tsx:70-73`).
- Authenticated (workspace) router — `useCreateWorkspaceAppRouter.tsx:101-259`, mounted by
  `WorkspaceApp.tsx:4-16`. Structure (top → leaf): `WorkspaceRouteObjectsProvider` →
  `WorkspaceAppProviders` → **`MinimalMetadataGate`** → `DefaultLayout` →
  `MainAppLayoutWithSidePanel` → the `main`-surface route objects
  (`useCreateWorkspaceAppRouter.tsx:110-135`). A parallel branch under the same providers
  keeps the auth pages reachable post-login (`VerifyEmail`, `ResetPassword`,
  `PlanRequiredSuccess`, `useCreateWorkspaceAppRouter.tsx:136-157`), and a `BlankLayout`
  branch holds onboarding (`Verify`, `WorkspaceActivation`, `CreateProfile`, `SyncEmails`,
  `InviteTeam`, `BookCall`, `PlanRequired`, `Authorize`;
  `useCreateWorkspaceAppRouter.tsx:158-253`).

`MinimalMetadataGate` blocking the workspace router until metadata loads is the single most
important architectural constraint on the port: **no authenticated screen renders before the
metadata store is hydrated** (`useCreateWorkspaceAppRouter.tsx:120`, gate component
`twenty-front/src/modules/metadata-store/components/MinimalMetadataGate.tsx`, loader
`metadata-store/hooks/useLoadMinimalMetadata.ts:17`).

Every route element is wrapped in `LazyRoute` (`app/components/LazyRoute.tsx`) around a
`React.lazy` import — code-splitting per route/page is already the upstream default, and the
onboarding branch additionally warms chunks through `lazyWithPreload` + a route `loader`
(`useCreateWorkspaceAppRouter.tsx:44-95`, preload list `useCreateWorkspaceAppRouter.tsx:86-95`).
The port must reproduce this laziness (§4.5).

### 1.2 Route → page map (`AppPath`, `twenty-shared/src/types/AppPath.ts`, 45 lines)

Workspace route objects, in file order (`app/routing/utils/createWorkspaceRouteObjects.tsx`),
each mapping an `AppPath` to a lazy page:

| URL pattern (`AppPath.ts`) | Line in `createWorkspaceRouteObjects.tsx` | Page (verified at tag) |
|---|---|---|
| `/getting-started` (`AppPath.GettingStarted`) | :103 | `pages/getting-started/GettingStartedPage` (:15) |
| `/workflows` (`WorkflowIndexPage`) | :111 | `pages/object-core/WorkflowCoreIndexPage` (:33) |
| `/objects/workflows` → redirect | :123-128 | `NavigatePreservingSearchAndHash` to `/workflows` |
| `/workflow/:coreWorkflowId` (`WorkflowCoreShowPage`) | :134 | `pages/object-core/WorkflowCoreShowPage` (:39) |
| `/agents` / `/agent/:agentId` (`AgentIndexPage`/`AgentShowPage`) | :146 / :154 | `pages/object-core/AgentCore*Page` (:21, :27) |
| `/` (`Index`) | :162 | `RecordIndexSkeletonLoader` (default landing) |
| `/objects/:objectNamePlural` (`RecordIndexPage`) | :166 | `pages/object-record/RecordIndexPage` (:45; reads `objectNamePlural` param `RecordIndexPage.tsx:16,29`) |
| `/object/:objectNameSingular/:objectRecordId` (`RecordShowPage`) | :178 | `pages/object-record/RecordShowPage` (:51) |
| `/page/:pageLayoutId` (`PageLayoutPage`) | :187 | `pages/page-layout/StandalonePageLayoutPage` (:57) |
| `/chat/:threadId?` (`AiChat`) | :195 | `pages/legacy-ai-chat/LegacyAiChatPage` (:63) |
| `/inbox/:threadId?` (`AiChatInbox`) | :203 | `pages/inbox/InboxPage` (:69) |
| `/home` (`Home`, mobile nav page) | :211 | `pages/mobile-home/MobileHomePage` (:75) |
| `/settings/*` (`SettingsCatchAll`, root stripped) | :92, :219 | `SettingsRouteOutlet` + children (`app/components/SettingsRoutes.tsx`, 1404 lines) |
| `/dpa` (`Dpa`, deep-link redirect) | :224 | `null` element (redirect-only) |
| `*` (`NotFoundWildcard`) | :228 | `pages/not-found/NotFound` (:81) |

Auth/onboarding URL set in `AppPath.ts:2-6,8-14,38,41`: `/verify`, `/verify-email`, `/welcome`,
`/invite/:workspaceInviteHash`, `/reset-password/:passwordResetToken`, `/workspace-activation`,
`/create/profile`, `/sync/emails`, `/invite-team`, `/plan-required`,
`/plan-required/payment-success`, `/book-call`, `/authorize`, `/dpa`.

**Parity-critical:** these URL patterns are the contract the Leptos router must mirror
(`RecordIndexPage`=`/objects/:plural`, `RecordShowPage`=`/object/:singular/:id`) so a shared
deep link, an SDK-generated link, or a Zapier action resolves identically on both apps (§4.6).

### 1.3 Object-record surfaces (the daily-driver cluster)

The record index dispatches on the active view type (`ViewType`,
`twenty-shared/src/types/ViewType.ts:1-11` = TABLE, KANBAN, CALENDAR, LIST, + 5 widget
types): TABLE → `RecordIndexTableContainer`, KANBAN → `RecordBoardContainer`, CALENDAR →
`RecordIndexCalendarContainer`, LIST → `RecordList`
(`object-record/record-index/components/RecordIndexContainer.tsx:54-75`). The module sizes
(record-table 337 files, record-field 418, record-board 105, record-calendar 45) and the
full field-type matrix are inventoried in wave 0 (`research/frontend-inventory.md:93-137`,
`462-465`) and hold at the tag.

Table geometry (all verified at tag, verbatim):
- row height **32** — `object-record/record-table/constants/RecordTableRowHeight.ts:1`;
- column min-width **104** — `.../constants/RecordTableColumnMinWidth.ts:1`;
- virtualization **slot-based**, not windowed-by-offset: `ROW_SLOT_COUNT = 240`
  (`record-table/virtualization/constants/RowSlotCount.ts:1`), page size
  `TABLE_VIRTUALIZATION_NUMBER_OF_RECORDS_PER_PAGE = 10`
  (`.../TableVirtualizationNumberOfRecordsPerPage.ts:1`), overscan
  `TABLE_VIRTUALIZATION_NUMBER_OF_OVERSCAN_PAGES = 7`
  (`.../TableVirtualizationNumberOfOverscanPages.ts:1`);
  the slot engine lives in `record-table/virtualization/` (19 components, 9 hooks:
  `useAllocateRowSlots`, `useLoadRecordsAtRowIndexes`, `useTriggerFetchPages`,
  `useProcessScrollTop`). **The row-slot model (240 reusable DOM slots, fetch-on-demand by
  row index, SSE-diff applied to slots) is the reference design the Leptos virtualizer copies
  (§5.3).**

Show page: `RecordShowPage.tsx` + `object-record/record-show/` (resource loader, breadcrumb,
page-layout renderer) — `research/frontend-inventory.md:116-119`.

### 1.4 Settings sections

`/settings/*` children come from `createSettingsRouteObjects` inside
`app/components/SettingsRoutes.tsx`; each is gated by a `settingsPermission` of type
`PermissionFlagType` (e.g. `WORKSPACE` :886, `AI_SETTINGS` :967, `CONNECTED_ACCOUNTS` :841,
`LAYOUTS` :997, `WORKSPACE_MEMBERS` :1006) and an optional `requiredFeatureFlag`. The full
path enum is `twenty-shared/src/types/SettingsPath.ts` (118 lines, verified at tag): profile
`:2-3`, experience `:4`, app-preferences `:5-11`, billing/usage `:12-18`, data-model
`objects*` `:21-33` (incl. `new-field/select` :24, `new-field/configure` :25, `new-index`
:26, `:fieldName` :28, `object-translations` :29, `object-indexes` :31, `objects/new` :33),
layout `:34`, members `:35-36` + roles `:110-113`, general/subdomain/custom-domain `:37-39`,
communications `:40-46`, AI `:49-56`, applications `:57-74`, developers + playground + API keys
+ webhooks `:80-86`, security/SSO `:88,90`, admin-panel `:92-108`, legal `:115-117`. The
wave-0 prose breakdown of these areas and their operations is
`research/frontend-inventory.md:48-73,216-227` and holds.

### 1.5 State model

**Jotai is the only client-state library** (no Recoil — confirmed `research/frontend-inventory.md:413`).
Atoms are made by the helpers in `twenty-front/src/modules/ui/utilities/state/jotai/utils/`
(verified present: `createAtomState.ts` def at :50, plus `createAtomFamilyState.ts`,
`createAtomComponentState.ts`, `createAtomSelector.ts`, `createAtomWritableFamilySelector.ts`,
`createIndexedDbBackedJotaiStorage.ts`, `createJotaiCookieStorage.ts`,
`routedFlowStateScopeRegistry.ts`). Two families matter most:

- **Metadata store** — `metadata-store/states/metadataStoreState.ts:56-63`: a
  `createAtomFamilyState<MetadataStoreItem, MetadataEntityKey>` keyed by entity kind, each
  value `{current[], draft[], status, currentCollectionHash?, draftCollectionHash?}`
  (`metadataStoreState.ts:40-45`), persisted (`storage: metadataStoreStorage`, `:62`,
  `localStorageOptions.getOnInit: true` `:63`; hydrated before render, `index.tsx:5,28`).
  The 25 tracked entity kinds are `ALL_METADATA_ENTITY_KEYS`
  (`metadataStoreState.ts:9-36`): objectMetadataItems, fieldMetadataItems,
  indexMetadataItems, views, viewFields, viewFilters, viewSorts, viewGroups,
  viewFilterGroups, viewFieldGroups, pageLayouts, pageLayoutTabs, pageLayoutWidgets,
  logicFunctions, navigationMenuItems, commandMenuItems, frontComponents, webhooks, roles,
  roleTargets, agents, skills, rowLevelPermissionPredicates,
  rowLevelPermissionPredicateGroups, applications.
- **Per-feature atoms** — auth (`auth/states/`: `currentUserState`,
  `currentWorkspaceState`, `currentWorkspaceMemberState`, `currentUserWorkspaceState`,
  `isCookieAuthActiveState`, `returnToPathState`; all imported in
  `apollo/hooks/useApolloFactory.ts:5-11`), client-config (`client-config/states/`, ~45 atoms
  fed by the REST `/client-config` payload — `client-config/utils/getClientConfig.ts:8`),
  side-panel stack, command-menu, selection, groups.

**Apollo cache** is separate and holds *server* data (records + the metadata *queries*).
Its cache is `InMemoryCache({ typePolicies: { RemoteTable: { keyFields: ['name'] } } })`
with `defaultOptions.watchQuery.fetchPolicy: 'cache-and-network'`
(`apollo/hooks/useApolloFactory.ts:53-64`).

**Theming** is CSS custom properties, not Emotion, at this tag (see correction in §2).
Light/dark are two generated stylesheets (`twenty-ui/src/theme/theme-light.css` 1059 lines,
`theme-dark.css` 1055 lines) that `ThemeProvider` toggles by adding a `.light`/`.dark`
class to `document.documentElement` (`twenty-ui/src/theme/ThemeProvider.tsx:14-19`) and set
`color-scheme` accordingly (`twenty-front/src/index.css:27-32`).

### 1.6 Corrections to wave-0 `frontend-inventory.md`

- The task brief describes twenty-front as "React + Apollo + **Emotion** + Jotai"; at 2.61.0
  the styling runtime is **Linaria**, not Emotion (`@linaria/core`/`@linaria/react` v7 —
  `twenty-front/package.json:49-50`) reading `themeCssVariables`, plus SCSS modules in
  twenty-ui (`twenty-ui/package.json:64`). No `@emotion` dependency is present in
  `twenty-front/package.json`. (Wave-0 `research/frontend-inventory.md:349` already names
  Linaria correctly.)
- The wave-0 doc lists 27 metadata entity kinds (`frontend-inventory.md:428`); the verified
  count at the tag is **25** (`metadataStoreState.ts:9-36`).
- Wave-0 assumed a `twenty-desktop` Tauri package. **There is no `twenty-desktop` and no
  Tauri anywhere at `twenty/v2.61.0`** — the only desktop product is `twenty-companion`,
  which is **Electron 44** (`twenty-companion/package.json:49` `"electron": "44.9.1"`; the
  string "twenty-desktop" survives only as a temp-dir name in
  `twenty-companion/scripts/package.mjs:10`). See §5.5 for what this means for the committed
  Tauri direction.

---

## 2. Visual design language (enough to compare against `http://127.0.0.1:3001`)

The authoritative prose spec is repo-root `DESIGN.md` (281 lines): "calm, dense,
keyboard-first workspace", "closer to Notion than Salesforce" (`DESIGN.md:4,31`), four
recognizable landmarks (record index, record page, settings, standalone pages,
`DESIGN.md:36`), light and dark equal (`DESIGN.md:38`). All numeric values below are read
from the generated tokens at the tag so a screenshot diff has an exact target.

### 2.1 Fonts & type scale

- Family: **Inter** for UI (`twenty-front/src/index.css:5` `font-family: 'Inter', sans-serif`;
  token `--t-font-family: Inter, sans-serif` `theme-light.css:185`), self-hosted weights
  400/500/600/700 imported in `twenty-front/src/index.tsx:8-11`; **DM Mono** for
  code/IDs/logs (`--t-code-font-family: DM Mono` `theme-light.css:262`, imported
  `index.tsx:6-7`).
- **Root font-size 13px** (`twenty-front/src/index.css:14` `html { font-size: 13px }`;
  restated `DESIGN.md:35,68`). So `--t-font-size-md = 1rem = 13px` (`theme-light.css:178`).
- Scale (`theme-light.css:175-181`): xxs `.625rem`, xs `.85`, sm `.92`, md `1`, lg `1.23`,
  xl `1.54`, xxl `1.85` rem. Weights (`:182-184`): regular 400, medium 500, semi-bold 600
  (`DESIGN.md:72-77`: titles semibold 24/20/16px, body 13px).

### 2.2 Spacing, radii, density

- **4px grid**: `--t-spacing-n = n×4px` (`DESIGN.md:86`), multiplicity token
  `--t-spacing-multiplicator: 4` (`theme-light.css:35`); verified anchors 0→`:36`, 1→`:37`,
  2→`:38`, 4→`:40`, 8→`:44`, 16→`:52`, 32→`:68`; the 0.5/1.5 sub-steps give 2px/6px
  (`frontend-inventory.md:350`). Half-steps and 25-31 fill 100-124px.
- **Control heights share a scale** (`DESIGN.md:91`): 32px for inputs / default buttons /
  menu rows, 24px for small buttons and toolbar icon buttons; icon-only buttons square.
- **Radii** (`theme-light.css:142-152`): xs 2, sm 4, md 8, lg 16, xl 20, xxl 40, pill 999px,
  rounded 100%; the never-double round variants sm-round 4 (`:145`), md-round 8 (`:146`),
  lg-round 12 (`:147`).
- **Squircle rule** (`DESIGN.md:108-113`, `theme-light.css:1027-1048`): under
  `@supports (corner-shape: squircle)` (Chromium 139+) radii **double** (xs 4 `:1044`, sm 8
  `:1045`, md 16 `:1046`, lg 32 `:1047`, xl 40 `:1048`) and `corner-shape:
  superellipse(2.2)` applies globally; nested radii stay concentric via
  `calc(var(--t-border-radius-md) - var(--t-spacing-1))` (`DESIGN.md:112`); round elements
  (avatars, chips, switch handles) use the `-round`/pill tokens that never double
  (`DESIGN.md:111`). **Port decision:** emit both CSS branches verbatim so the screenshot
  matches on Chromium (`--t-zoom` interface scale also multiplies uniformly,
  `index.css:22-24`).
- **Table density**: row 32px (`RecordTableRowHeight.ts:1`), horizontal cell margin+padding
  both 8px (`--t-table-horizontal-cell-margin`/`-padding` `theme-light.css:72,74`), checkbox
  column 32px (`--t-table-checkbox-column-width` `:73`).
- **Shadows** float only (`DESIGN.md`, `frontend-inventory.md:373-379`): color/light/sidebar/
  bottomPanel/strong/underline/superHeavy from `twenty-ui/design-tokens/boxShadow.ts`,
  applied to dropdowns, popovers, dialogs only (`theme-light.css:153-163`).

### 2.3 Colors (light + dark, Display P3)

Tokens are `color(display-p3 …)`; the exact numeric ramp is what a pixel/label diff checks.

| Role | Light (`theme-light.css`) | Dark (`theme-dark.css`) |
|---|---|---|
| background primary | `1 1 1` `:98` | `0.09 0.09 0.09` `:98` |
| background secondary | `0.988` `:99` | `0.106` (inverted scale) |
| background tertiary | `0.945` `:100` | `0.114` `:100` |
| background quaternary | `0.922` `:101` | `0.133` |
| font primary | `0.2` `:167` | `0.922` `:163` |
| font secondary | `0.4` `:168` | `0.702` |
| font tertiary | `0.6` `:169` | `0.506` |
| border strong | `0.839` `:134` | `0.282` `:134` |
| border medium | `0.922` `:135` | `0.133` `:135` |
| accent primary/secondary | `0.831 0.87 1` `:79-80` | `0.163 0.22 0.439` `:79-80` |
| accent ramp endpoints | accent1 `0.992 0.992 0.996` `:85`, accent12 `0.133 0.175 0.348` `:96` | accent1 `0.068 0.074 0.118` `:85`, accent12 `0.848 0.881 0.99` `:96` |
| accent9 = product blue | `0.276 0.384 0.837` `:93` (identical in dark `:93`) | same `:93` |

- The single accent is that blue (`DESIGN.md:35,58`; lint rejects hex/rgb — colors chosen by
  role from semantic tokens, `DESIGN.md:42`).
- Palette: **25 hues** (`--t-color-{hue}{1..12}` + base `--t-color-{hue}`), names enumerated in
  `twenty-ui/src/theme/constants/MainColorNames.ts` = red, ruby, crimson, tomato, orange,
  amber, yellow, lime, grass, green, jade, mint, turquoise, cyan, sky, blue, iris, violet,
  purple, plum, pink, bronze, gold, brown, gray (verified 25, matches
  `frontend-inventory.md:372-373`). Light ramps light→dark, dark inverted
  (`frontend-inventory.md:373`). `theme-light.css` contains 753 `t-color-` declarations.

### 2.4 Layout metrics (constants, all verified at tag)

`APP_HEADER_HEIGHT = 48` (`modules/ui/layout/constants/AppHeaderHeight.ts:1`); side panel
`{min:320, max:600, default:400}` (`modules/side-panel/constants/SidePanelConstraints.ts:1-5`);
navigation drawer `{min:180, max:350, default:220}` + collapsed 48
(`modules/ui/navigation/navigation-drawer/constants/NavigationDrawerConstraints.ts:1-5`);
record index row 32 / min-width 104 (§1.3); mobile breakpoint `MOBILE_VIEWPORT = 768`
(`twenty-ui/src/theme/constants/MobileViewport.ts:3`, used in media queries e.g.
`PageHeader.tsx:45`); modal width tokens sm 300 / compact 360 / md 400 / lg 53% / xl
1200×800 / fullscreen 100dvw×100dvh (`theme-light.css:13-20`).

### 2.5 Component inventory (the set to reproduce, with the token each consumes)

Two source layers at the tag: published primitives `twenty-ui/src/primitives/**` (618 files
on **@base-ui/react 1.9** + SCSS — `twenty-ui/package.json:80-81`) and app components
`twenty-ui/src/components/**` (503 files). Verified primitive folders: input (Button,
ButtonGroup, Input, InputGroup, NumberField, Textarea, Checkbox, Radio, RadioGroup, Switch,
Slider, Autocomplete, Field), data-display (Avatar, Badge, Chip, ColorSample, Status, Tag),
feedback (Banner, Loader, Progress, ProgressBar, ProgressRing, Skeleton), surfaces (AlertDialog,
Card, Dialog, Popover, Tooltip), navigation (Breadcrumb, ListItem, Tabs), typography, layout
(Collapsible, ResizeHandle, Separator), accessibility. App components add MainButton,
LightButton, IconButton, SearchInput, DatePicker, SegmentedControl, Dropdown/MenuItem,
TabButton, CountPager, Toast/Toaster, Callout, InlineBanner, CodeEditor (Monaco).

Key APIs to mirror (`research/frontend-inventory.md:338-345`, unchanged at tag): **Button**
variant `solid|outline|ghost|soft|link` × color `neutral|accent|danger|success|inherit` ×
size `sm`(24px)`|md`(32px), `solid`+`inherit` throws, `elevated`/`loading`/icons; **Chip**
variant `solid|soft|ghost|plain|outline`, shape `square|round`, color primary|secondary;
**Tag** color = any of the 25 theme colors + transparent; **Select**, **Modal/Dialog**
(size = the `--t-modal-size-*` set), **SidePanel** (drawer 48px top bar = `APP_HEADER_HEIGHT`,
width from `SIDE_PANEL_CONSTRAINTS`), **TabList/TabButton**.

Icons: base set **Tabler** (`@tabler/icons-react` ^3.31 `twenty-ui/package.json:83`),
re-exported through `twenty-ui/src/icon`; the 674-line name→icon map is
`twenty-ui/src/icon/constants/TwentyIconDictionary.ts` (verified 674 lines) with per-field
illustration icons and object icons (Person→IconUser, Company→IconBuildingSkyscraper, etc.,
`frontend-inventory.md:401-407`). Default icon 16px, 14px in dense cells, stroke 1.6
(`theme-light.css:6-9,31-33`; `DESIGN.md:82`).

---

## 3. Data layer

### 3.1 Two endpoints, two clients (name-vs-URI correction)

Upstream runs **two `ApolloClient` instances** built by the same `ApolloFactory`
(`apollo/services/ApolloFactory.ts:311` `new ApolloClient({cache, link, defaultOptions, devtools})`).
The URIs are **inverted relative to their names** — flag this because it trips readers:

- The **default** client, mounted by `ApolloProvider` and used by `useApolloClient()`,
  targets **`/metadata`**: `ApolloProvider.tsx:8`
  (`uri: \`${REACT_APP_SERVER_BASE_URL}/metadata\``), wrapped around the whole app by
  `SharedAppProviders.tsx:3,13`. This is the client for the metadata store, auth/user,
  navigation/command-menu, settings, page-layout — everything against the *metadata* schema.
- The **"core"** client, mounted by `ApolloCoreProvider`
  (`object-metadata/components/ApolloCoreProvider.tsx:10-18`) and reached through
  `useApolloCoreClient()` (`object-metadata/hooks/useApolloCoreClient.ts:5-20`), takes the
  factory's **default URI `/graphql`** (`useApolloFactory.ts:52`, `ApolloCoreProvider` passes
  no `uri`). This is the *dynamic workspace* schema — the generated record operations
  (`useFindManyRecords.ts:9,55`, group-by/aggregate hooks `record-index/hooks/*.ts:1`).

So: **`/metadata` = core schema (metadata + auth + settings + SSE + subscriptions);
`/graphql` = the per-workspace dynamic record schema.** REST is a third surface:
`REST_API_BASE_URL = ${SERVER_BASE_URL}/rest` (`apollo/constants/RestApiBaseUrl.ts:3`),
reached through an Apollo `rest` directive link (`ApolloFactory.ts:316-333`
`isRestOperation`); client-config is plain REST `${SERVER}/client-config?v=…`
(`client-config/utils/getClientConfig.ts:8`). This matches the server-side route inventory in
wave 0 (`research/graphql-rest-api.md:49-51`: `/graphql` carries the heavier middleware
chain, `/metadata` is auth + workspace hydrate) — **except** the wave-0 note at
`graphql-rest-api.md:166` that subscriptions ride core `/graphql`; the front's SSE client
actually connects to **`/metadata`** (`SSEClientEffect.tsx:61-62`
`createClient({url: \`${REACT_APP_SERVER_BASE_URL}/metadata\`})`, library `graphql-sse`
`^2.5.4` `twenty-front/package.json:109`). **Nudge's `/metadata` endpoint must therefore
serve the GraphQL-over-SSE subscription transport** — record this as a wave-4 server
requirement (§3.5).

Transport links, in order (`ApolloFactory.ts:296-306`):
`supersededSessionRetryLink → errorLink → authLink → (extras) → logger → retryLink →
streamingRestLink → restLink → uploadLink`. `authLink` carries the bearer/renewal;
`credentials:'include'` on REST/upload links means session cookies are the primary auth
(`frontend-inventory.md:247-248`; cookie contract ported in wave 2, `rust/MAP.md:76-96`).

### 3.2 Query/mutation families (with file refs)

**Metadata (`/metadata`)**
- Minimal bootstrap: `FindMinimalMetadata` — `metadata-store/graphql/queries/findMinimalMetadata.ts:3-4`;
  returns `objectMetadataItems{id,nameSingular,namePlural,labelSingular,labelPlural,icon,isActive,isSystem,isRemote}`,
  `views{id,type,key,objectMetadataId}`, `collectionHashes{collectionName,hash}` (`:4-28`).
- Full reload of a stale kind: `useLoadStaleMetadataEntities.ts` drives, per group — objects:
  `FIND_MANY_OBJECT_METADATA_ITEMS` (`object-metadata/graphql/queries.ts:4-6` `query ObjectMetadataItems`,
  fragment `OBJECT_METADATA_FRAGMENT` `object-metadata/graphql/fragment.ts`); views group:
  `FindAllViews/FindTableWidgetViews/FindFieldsWidgetViews`; page-layout group:
  `FindAllRecordPageLayouts/FindAllRecordFormPageLayouts`; plus
  `FindManyCommandMenuItems/FindManyNavigationMenuItems/FindManyFrontComponents/FindManyLogicFunctions`
  (import list `useLoadStaleMetadataEntities.ts:12-22`). Responses are flattened to flat
  maps (`splitObjectMetadataGqlResponse.ts:14-51`), matched against
  `collectionHashes`, and merged current→draft (`getDraftOrCurrentMetadataItems.ts`).
- Metadata mutations (data-model editing) go through the same `/metadata` client:
  `CreateOneObjectMetadataItem`, `UpdateOneFieldMetadataItem`, `CreateOneIndexMetadataItem`,
  `CreateValidationRule`, view CRUD (`CreateView/UpdateView/DestroyView`,
  `CreateManyViewFields`, filter/sort/group CRUD), role/API-key/webhook ops
  (`research/frontend-inventory.md:164-169,224-227`).

**Records (`/graphql`, dynamically generated per workspace)**
- The list query is **generated at runtime from metadata**, not hand-written:
  `generateFindManyRecordsQuery.ts:26-58` emits `query FindMany<Plural>($filter:
  <Singular>FilterInput, $orderBy: [<Singular>OrderByInput], $lastCursor, $limit, $offset)`
  selecting `edges{node<…>,cursor}`, `pageInfo{hasNextPage,hasPreviousPage,startCursor,endCursor}`,
  `totalCount`; the node projection is built by `mapObjectMetadataToGraphQLQuery.ts:32`
  (composite → sub-fields, relation → join columns, morph → per-target fields,
  `frontend-inventory.md:452-466`).
- Record mutations likewise generated: `generateCreateOneRecordMutation.ts:12,38`
  (`mutation CreateOne<Singular>($input: <Singular>CreateInput!)`), plus the
  Update/UpdateMany/Delete/Destroy/Restore/Merge family
  (`generateUpdateOneRecordMutation.ts`, `generateUpdateManyRecordsMutation.ts`,
  `object-metadata/utils/generate*`, `frontend-inventory.md:126-137`).
- Search: `Search` query (generated doc `SearchDocument`), variables
  `$searchInput,$limit,$after,$excludedObjectNameSingulars,$includedObjectNameSingulars,$filter`,
  returning `edges{node{recordId,objectNameSingular,objectLabelSingular,label,imageUrl,tsRankCD,tsRank},cursor},
  pageInfo{hasNextPage,endCursor}` (`generated/graphql.ts:370`; consumed
  `object-record/hooks/useObjectRecordSearchRecords.ts:41`).
- Aggregate/GroupBy (`useAggregateRecords`, `useGroupByRecordsQuery`, `frontend-inventory.md:136`),
  counts (`OBJECT_RECORD_COUNTS`), duplicates (`FindDuplicateRecords`), and permissions
  (`GetRecordPermissions`) round out the read family.

**Auth / session (`/metadata`)** — operation set enumerated and already ported server-side in
wave 2: SignIn, SignUp, SignUpInNewWorkspace, SignOut, getLoginTokenFromCredentials,
getAuthTokensFromLoginToken, getAuthTokensFromOTP, getAuthTokensFromSsoExchangeToken,
VerifyEmail*, UpdatePasswordViaResetToken, EmailPasswordResetLink, ResendEmailVerificationToken,
CheckUserExists, GetPublicWorkspaceDataByDomain, impersonate/stop-impersonation
(`frontend-inventory.md:239-252`; REST mirrors in `rust/MAP.md:145-171`). `GetCurrentUser`
(`users/graphql/queries/getCurrentUser.ts:6`) returns user + workspace + permissionFlags +
objectRecordsPermissions + feature flags.

### 3.3 Caching strategy in Rust — recommendation

**Recommendation: a query-level cache with per-object normalized side-indexes, NOT a full
Apollo-style garbage-collected normalized cache.** Rationale:

1. Upstream's own record reads are *not* normalized-merge driven: they use **generated
   connection queries** keyed by variables and patch them imperatively. The optimistic layer
   (`apollo/optimistic-effect/utils/triggerUpdateRecordOptimisticEffect.ts:10`,
   `triggerAttachRelationOptimisticEffect.ts`, `buildSortedConnectionEdges.ts`) writes
   *into known query results by cursor/edge*, and SSE events patch the cache for the bound
   queries (`useApplyRecordOperationToRows.ts`-style slot updates `record-table/virtualization/hooks/`).
   A record is reached by (objectName, id) only on the show page. So the true read model is
   "N materialized query responses + a thin id→record index", which a query-level cache
   reproduces directly; a full normalizer would re-derive an abstraction the app never relies
   on, at real wasm cost.
2. The metadata store is already the *other* half and is explicitly **flat keyed maps with
   collection hashes** (`metadataStoreState.ts:40-63`,
   `ALL_METADATA_ENTITY_KEYS`), i.e. normalized by (entityKind, id) with hash-based staleness.
   Reusing that same flat-map machinery for records (an `RustHashMap<RecordId,Record>` side
   index + `Vec`-ordered edge lists per query) keeps one mental model and one invalidation
   path.

Design (crate `front-core`, §4): a `QueryCache` keyed by a stable `QueryKey = {operation,
normalizedVariables, targetSchema}` holding `{data: serde_json::Value (or typed), status,
timestamp, subscriptionId}`; a `RecordIndex` side-map `{objectName → {id → Rc<Record>}}`
that the query layer reads through so a single record write can fan out to every cached query
that references that id (this is what makes optimistic edits and SSE patches O(references)
not O(all)). TTL mirrors `cache-and-network` (`useApolloFactory.ts:60-64`) — always revalidate
on mount, serve cache-first while in flight. Normalization is *logical* (id index), not
*structural* (no field-level cache fragments), which removes Apollo's
`InMemoryCache` typePolicy complexity (`useApolloFactory.ts:53-56`) while keeping every
behaviour the UI depends on.

### 3.4 Realtime / invalidation model

Single SSE stream per session: `graphql-sse` client on `/metadata` (`SSEClientEffect.tsx:61`),
one `eventStreamId` (`sse-db-event/hooks/useTriggerEventStreamCreation.ts`, state
`sseEventStreamIdState`). Queries bind to the stream by **`AddQueryToEventStream`** mutation
(`ADD_QUERY_TO_EVENT_STREAM_MUTATION`, `addQueryToEventStream(input: AddQuerySubscriptionInput!)`,
`sse-db-event/graphql/mutations/AddQueryToEventStreamMutation.ts:3-5`, with a matching remove).
The subscription `OnEventSubscription` (`ON_EVENT_SUBSCRIPTION`,
`sse-db-event/graphql/subscriptions/OnEventSubscription.ts:3-47`) delivers three event kinds —
`objectRecordEventsWithQueryIds{objectRecordEvent{action,objectNameSingular,recordId,userId,
workspaceMemberId,properties{updatedFields,before,after,diff}},queryIds}`,
`metadataEvents{type,metadataName,recordId,updatedCollectionHash,properties{…}}`,
`queueJobEvents{…}` (`:7-45`). Dispatch is a fan-out to browser events
(`dispatchQueueJobEventsFromSseToBrowserEvents.ts`,
`turnSseObjectRecordEventToObjectRecordOperationBrowserEvent.ts`) which then
(a) patch the query cache for the listed `queryIds` and (b) for metadata, bump the relevant
`currentCollectionHash`; a metadata change whose hash moved triggers a targeted reload via
`useLoadStaleMetadataEntities` (`metadata-store/hooks/useLoadStaleMetadataEntities.ts`), and a
full reconnect triggers a debounced `resyncMetadataStore` (`SSEClientEffect.tsx:24-35`,
`useResyncMetadataStore.ts:8-11`). **Port contract:** the Leptos SSE worker parses
`OnEventSubscription` frames identically; record events update `QueryCache`+`RecordIndex`;
metadata events update the metadata store by hash; reconnection resyncs.

### 3.5 Optimistic updates

Field edit → optimistic cache write → `updateOne<X>` mutation → SSE confirm
(`frontend-inventory.md:509-514`); the write engine is `triggerUpdateRecordOptimisticEffect.ts:10`
plus the relation attach/detach (`triggerAttachRelationOptimisticEffect.ts`,
`triggerDetachRelationOptimisticEffect.ts`) and batch (`triggerUpdateRecordOptimisticEffectByBatch.ts`),
group-by-aware (`buildSortedConnectionEdges.ts`). Rollback on error restores the pre-mutation
snapshot from the query cache. Port: every mutation carries an `OptimisticEffect` closure run
against `QueryCache`/`RecordIndex` before dispatch, reverted on rejection (§4).

Server contract this requires (hand to wave 4): `/metadata` must expose the
`AddQueryToEventStream`/`RemoveQueryFromEventStream` mutations, the `onEventSubscription`
GraphQL-over-SSE field, and the `updatedCollectionHash` in metadata events, and `/graphql`
must accept the generated `FindMany`/`CreateOne`/`UpdateOne`/… operations with
`edges/pageInfo/totalCount` shaping (`research/graphql-rest-api.md` already pins the
workspace resolver that produces this shape).

---

## 4. Leptos architecture proposal

### 4.1 Crate layout (new crates under `crates/`, disjoint from backend waves)

The workspace already reserves exactly this: "crates/ui (Leptos, wasm32) and crates/desktop
(Tauri) … join the workspace once their toolchains are installed" (`Cargo.toml:4-5`), and
`README.md:150-151` names `crates/ui` and `crates/desktop`. To keep units conflict-free and
wasm/native separation clean, split the UI crate:

- **`crates/front-core`** (`nudge-front-core`) — no DOM, wasm+native, the logic layer:
  GraphQL client (transport, links, auth header/cookie, two endpoints), `QueryCache` +
  `RecordIndex` (§3.3), metadata store (flat maps + collection-hash diff, §1.5), the
  `OnEventSubscription` SSE decoder (§3.4), optimistic-effect engine (§3.5), client-config,
  field-metadata → GraphQL projection builder (`mapObjectMetadataToGraphQLQuery` port §3.2),
  and the generated-query builder (`generateFindManyRecordsQuery` port). This is the crate a
  future CLI/test harness can reuse without a browser.
- **`crates/front-ui`** (`nudge-front-ui`) — design-system crate: the token sheets (§2) as a
  build-time-generated CSS asset + a `theme_css!()` include, and the Leptos component
  primitives mirroring twenty-ui (§2.5: Button, Input, Select, Chip, Tag, Avatar, Modal,
  SidePanel, TabList, Table, Dropdown, Toast, Icon set). No app logic.
- **`crates/front-app`** (`nudge-front-app`) — the Leptos app: routers (§4.6), pages/features,
  and the `Trunk.toml` index. Depends on front-core + front-ui. Submodules mirror upstream
  module boundaries: `app/` (router+providers+layouts), `auth/`, `metadata/`, `object_record/`
  (table/board/calendar/list/show/field), `views/`, `navigation/`, `command_menu/`,
  `side_panel/`, `settings/`, `page_layout/`.
- **`crates/desktop`** (`nudge-desktop`) — Tauri 2 shell (§5.5), Rust main process.
- **`crates/front-serve`** (`nudge-front-serve`) — the static asset server for web deploys
  (§5.4): HTTP/2 + brotli + the SPA catch-all. A separate crate so it never edits
  `crates/server` (backend waves own that); the production composition later mounts it beside
  `nudge-server`.

`nudge-core` (`crates/core`) is reused as-is for shared primitives (identifiers, config,
error) — the UI crates depend on it exactly as `crates/server` does (`Cargo.toml:14`).

### 4.2 wasm target & toolchain

- Targets already present: `wasm32-unknown-unknown` added, rustc/cargo 1.98.1
  (`rust/MAP.md:13`). Not yet installed: `trunk`, `cargo-tauri` (`rust/MAP.md:14`) — **wave-5
  prerequisite step**: `cargo install trunk@0.22.2 cargo-install cargo-binstall` and the tauri
  CLI. Current pins for the lockfile: **trunk 0.22.2** (crates.io max_stable), Leptos **0.8.x
  stable line (0.8.23)** — the workspace pins the last stable major; 0.9.0 is rc-only and is
  *not* adopted for a product wave (`rust-anything`/MAP rule: each wave builds, tests, runs —
  an rc framework is disqualifying). `leptos_router 0.8`, `leptos_meta 0.8`,
  `wasm-bindgen 0.2.130`, `web-sys`/`js-sys` matching. Tauri **2.12.7** (§5.5).
- `front-core`/`front-ui` compile for both `wasm32-unknown-unknown` and `x86_64-apple-darwin`
  (native) so unit tests run in-process without a browser; DOM-only code is behind
  `#[cfg(target_arch="wasm32")]` and the `leptos/dom` feature.
- Edition 2021, rust-version 1.85 (`Cargo.toml:8-12`) — Leptos 0.8 is compatible.

### 4.3 trunk configuration (per `crates/front-app`)

```
# Trunk.toml (illustrative; committed in U3)
[build] target = "index.html"
[serve] address = "127.0.0.1" port = 8080
[tools.wasm-bindgen] version = "0.2.130"
# release profile: reference-optimized wasm, brotli'd by front-serve (§5.4)
```

`index.html` mirrors upstream's structure (`twenty-front/index.html`: env-config script, PWA
manifest, `#root`) but loads the trunk-emitted wasm bundle instead of a Vite module, and the
generated `theme-light.css`/`theme-dark.css` + Inter/DM-Mono fontsheets (§2.1) as
`<link>`/`@font-face` assets. **The `@fontsource` webfonts are vendored as static assets** (no
npm in the Rust build). `trunk build --release` is the artifact step; `Trunk.toml` hooks run
the token generator (`§2`) as a `pre_build` so CSS is never stale.

### 4.4 SSR vs CSR — follow the committed choice: **CSR**

The repo commits to CSR twice: ADR-008 "The UI is a Leptos **CSR** app built with `trunk`"
(`rust/DECISIONS.md:45-46`) and the architecture box "Leptos UI (wasm, CSR)"
(`README.md:135-136`). This also matches upstream's own shape (a Vite SPA that
`ReactDOM.createRoot(...).render(<App/>)` client-side, `twenty-front/src/index.tsx:19-30`),
and it is the correct call for the port's stated performance story (one static bundle + the
compiled server, `README.md:28-30`, no per-request render). **Decision: CSR**,
Leptos `hydrate`/SSR features unused; the only server-side rendering is `front-serve`
returning the static `index.html`. Rationale to record: (a) parity is a client/asset contract
(the server is a pure API, `rust/DECISIONS.md:46` "the server stays usable without the desktop
app"), (b) SSR would couple the UI to the axum server build and break the "static assets"
deployment model, (c) the reference app is itself a CSR SPA, so a CSR port is the closest
behavioral match including auth-redirect and cookie flow (§3.1 `credentials:'include'`).

### 4.5 Code-splitting, preconnect, prefetch (mirroring upstream, wasm-native)

- **Per-route splitting:** every upstream route element is `React.lazy`/`LazyRoute`
  (§1.1). Leptos has no per-component dynamic wasm import in the Vite sense, so the port
  splits at the **module-chunk** level: `front-app` is compiled to a base bundle (router,
  layout, auth, metadata, table) plus lazy feature bundles. Concretely: keep the hot path
  (shell + record table) in the primary wasm, and gate the heavy, rarely-visited surfaces
  (settings data-model, workflow diagram, page-layout editor, AI chat, spreadsheets) behind
  Leptos `<Suspense>` + a second wasm module loaded on first navigation. Default decision if
  multi-module wasm proves costly on the pinned toolchain: **one wasm bundle + JS-level lazy
  of non-wasm assets** (Monaco, chart libs) and route-level *data* prefetch, and document it
  as an ADR-adjacent deviation. The parity gate is first-paint and time-to-interactive on the
  record index, not chunk count.
- **Preconnect:** upstream `index.html` has **no** preconnect (verified: the only `<link>`s
  are apple-touch-icon and manifest, `twenty-front/index.html:12-13`) — so preconnecting the
  API origin is a **port improvement**, not parity. Emit
  `<link rel="preconnect" href="{SERVER_BASE_URL}">` + `dns-prefetch` in the generated
  `index.html` head (§4.3) for `/metadata` and `/graphql`.
- **Metadata prefetch before boot:** replicate the `hydrateMetadataStore()`-before-render
  ordering (`index.tsx:5,28`) — `front-app` boots, fires `FindMinimalMetadata`
  (`findMinimalMetadata.ts:3`) + `GetCurrentUser`, hydrates the persisted metadata store from
  IndexedDB/localStorage (port of `metadataStoreStorage`, `metadataStoreState.ts:62`), and
  releases `MinimalMetadataGate` (`useCreateWorkspaceAppRouter.tsx:120`) once the minimal set
  is present; the full per-kind load (`useLoadStaleMetadataEntities.ts`) then runs by hash
  diff. This is the equivalent of the onboarding `preloadOnboardingPages` route `loader`
  warm-up (`useCreateWorkspaceAppRouter.tsx:86-95,163`).

### 4.6 Router design (mirrors upstream URLs, `leptos_router`)

Leptos `leptos_router` declarative routes mapped 1:1 to `AppPath` (§1.2) so URL parity holds:
`/welcome`, `/invite/:workspaceInviteHash`, `/verify`, `/verify-email`,
`/reset-password/:passwordResetToken`, `/workspace-activation`, `/create/profile`,
`/sync/emails`, `/invite-team`, `/book-call`, `/plan-required`, `/plan-required/payment-success`,
`/authorize`, `/objects/:object_name_plural`, `/object/:object_name_singular/:object_record_id`,
`/page/:page_layout_id`, `/workflows`, `/workflow/:core_workflow_id`, `/agents`,
`/agent/:agent_id`, `/chat/:thread_id?`, `/inbox/:thread_id?`, `/getting-started`, `/home`,
`/settings/*`, `/dpa`, `*` → NotFound (source: `AppPath.ts:2-45`, route lines in
`createWorkspaceRouteObjects.tsx:103-228`). The two-router login split (§1.1) becomes a route
guard: unauthenticated → auth route set; authenticated → workspace route set behind a
`MinimalMetadataGuard` component that blocks render until the store is hydrated
(port of `MinimalMetadataGate`, `useCreateWorkspaceAppRouter.tsx:120`). Deep-link redirects
(`/objects/workflows → /workflows`, `createWorkspaceRouteObjects.tsx:123-128`; root
`RootDeepLinkRedirect`, `useCreateRootAppRouter.tsx:70-73`) are explicit `<Redirect>` routes.

Settings sub-routes are the `SettingsPath` enum (`SettingsPath.ts:1-118`) mounted under
`/settings/`, each wrapped in a permission gate reading `PermissionFlagType` from
`GetCurrentUser` (`SettingsRoutes.tsx:841-1006`). Main-vs-side-panel surface split (§1.1,
`getWorkspaceRouteObjectsForSurface.tsx:15-16`) is reproduced by a route `handle`/metadata map
marking which paths open in the side panel (`MAIN_AND_SIDE_PANEL` tags at
`createWorkspaceRouteObjects.tsx:116,138,170,181`).

---

## 5. Multithreading plan (explicit user requirement)

Upstream runs **single-threaded**: there are **no production Web Workers** in
`twenty-front` at the tag — the only `new Worker(` is Monaco's inert worker in a settings
*story* file (`settings/developers/components/__stories__/GraphQLPlayground.stories.tsx:31`);
no `@tanstack/react-virtual`, no `react-window` — virtualization is bespoke DOM-slot recycling
(§1.3). So every "use threads" decision below is a **port improvement over the reference**,
documented as such; parity is behavioral+visual, not thread-count.

### 5.1 Worker strategy (three tiers)

1. **wasm workers first (default).** CPU-bound, off-critical-path work moves to dedicated
   workers via **`gloo-worker` 0.7** (verified current), spawned from `front-core`:
   - the **SSE stream parser/decoder** (§3.4) — JSON frame decode + event fan-out off the UI
     thread so a burst of record events never blocks scroll;
   - the **GraphQL projection/query builder + response normalizer** (§3.2/§3.3) for large
     pages (metadata-heavy objects, 100s of fields) and `mapObjectMetadataToGraphQLQuery`
     over wide schemas;
   - **CSV import/export + full-text search ranking** preprocessing (wave 6 features, worker
     seam reserved now in `front-core/src/workers/`).
   Leptos 0.8 is single-threaded on wasm32; the worker boundary is message-passing
   (`gloo-worker` scoped tasks), not shared memory, which matches the actor/event dispatch
   already present upstream (`dispatchQueueJobEventsFromSseToBrowserEvents.ts`).
2. **`wasm-bindgen-rayon` / shared-memory pool — gated, not default.** True shared-memory
   threads (`wasm threads + atomics`) require COOP/COEP cross-origin isolation headers, which
   break the `credentials:'include'` cookie model on cross-origin API calls (§3.1) and are
   still unreliable on Safari. **Decision: do not enable in wave 5**; keep rayon available
   behind a build feature for the desktop Tauri webview only (where same-origin and COEP are
   controllable, §5.5). Rationale: the reference never needed threads, and the record-table
   budget is met by tier-1 + slot virtualization (§5.3).
3. **JS-side offload where wasm can't reach.** DOM measurement (column auto-size, sticky
   headers) and IntersectionObserver scroll bookkeeping stay on the main thread but in
   `requestAnimationFrame` batches — Leptos effects, not a worker, because a worker cannot
   touch layout.

### 5.2 Main-thread budget & 10k+ row table virtualization

The hot loop is the record table. Budget target: **≥ 55 fps scroll and < 100 ms
interaction latency at 10 000 rows.** Mechanism = port of the upstream slot engine
(`record-table/virtualization/`, §1.3): `ROW_SLOT_COUNT = 240` reusable row elements
(`RowSlotCount.ts:1`), 10-record fetch pages (`…NumberOfRecordsPerPage.ts:1`), 7-page
overscan (`…NumberOfOverscanPages.ts:1`). In Leptos: a fixed pool of 240 keyed row components
bound to a `create_slice`-style virtual list over the `QueryCache` edge array; scroll handler
(`useProcessScrollTop` port) reassigns slot→row-index and requests only the pages entering the
window; cell content is metadata-driven (`FieldDefinition`,
`frontend-inventory.md:452-466`) and memoized per (recordId, fieldId) so a scroll repositions
DOM without re-rendering cells. Because rows are fixed 32px (`RecordTableRowHeight.ts:1`), no
measure pass is needed — pure offset math, cheap enough to keep off any worker. The worker in
tier-1 only preps *page payloads* (normalize 10 new records) so the main thread never parses a
network body inline.

### 5.3 Prefetch pipeline for the table

Cursor infinite-scroll (`generateFindManyRecordsQuery.ts:36-38` after/first, before/last, offset) with
a lookahead of ~2 pages beyond overscan, issued from `useTriggerFetchPages` port; on `cache-and-network`
(`useApolloFactory.ts:60-64`) every visible page is served from `QueryCache` while the refetch
runs, so a re-scroll is instant (matches upstream `cache-and-network`).

### 5.4 HTTP/2 + brotli static serving (Rust side)

`crates/front-serve` serves the trunk release bundle with **tower-http 0.7.1** (verified
current): `ServeDir` + `SetHeader`/`Compression` middleware. Concrete defaults (no TBD):
- **brotli** for `text/*`, `application/wasm`, `application/javascript` (feature
  `compression-brotli`); gzip fallback; assets ≥ 1 KB compressed, immutable
  `Cache-Control: public, max-age=31536000, immutable` for content-hashed filenames,
  `no-store` for `index.html` (mirrors upstream's `Cache-Control: no-store` on the SPA
  document, observed on `twenty-ref` `GET /` headers).
- **HTTP/2** via the axum hyper server over rustls (the workspace is rustls-only, no OpenSSL —
  `Cargo.toml:29`, ADR-013 rationale `rust/DECISIONS.md:77`); ALPN `h2`. TLS terminates here in
  prod; in dev, trunk's own server is plain HTTP/1.1 (fine for parity).
- `application/wasm` with `Content-Type: application/wasm` + streaming; `Link: rel=preload`
  for the primary `.wasm` + fonts; the preconnect head from §4.5.
- **SPA catch-all**: unknown GET paths → `index.html` (the documented "SPA catch-all is the
  wave 5 gap" at `rust/HANDOFF.md:72`; today `crates/server` returns a NestJS-style 404 JSON on
  its fallback, `crates/server/src/routes/mod.rs:19-38`, which is *not* what a browser client
  wants — so `front-serve` provides the HTML catch-all and the API stays on `crates/server`).
  `parity-compare.py` currently lists this catch-all as the wave-5 gap (`rust/HANDOFF.md:72`).

### 5.5 Tauri 2 desktop shell (Rust main process) — with an upstream correction

**Correction to the premise.** The instruction asks to "verify upstream uses Tauri
(twenty-desktop)". It does not, at the pinned version. At `twenty/v2.61.0`:
- there is **no `twenty-desktop` package** (`git ls-tree twenty/v2.61.0 packages/` lists no
  such dir; the repo's whole git history from the 2022-12-01 initial commit has never contained
  `packages/twenty-desktop` — `git log --all -- packages/twenty-desktop` returns 0 commits);
- there is **no Tauri** in the tree (`git grep -l -i tauri twenty/v2.61.0` → none; no
  `src-tauri` anywhere in history);
- the **only** desktop product is **`twenty-companion`**, which is **Electron 44.9.1**
  (`twenty-companion/package.json:49`; scripts `start: electron .`
  `twenty-companion/package.json` `start`, packaging via `@electron/packager`
  `:40`). The leftover string "twenty-desktop-package" at
  `twenty-companion/scripts/package.mjs:10` is a temp-dir name from the pre-rename era, not a
  Tauri app.

**Therefore** the committed `crates/desktop` Tauri 2 shell (ADR-008 `rust/DECISIONS.md:45`,
`README.md:80,151`, `Cargo.toml:4-5`) is a **Nudge-original** desktop wrapper, not a port
of an upstream desktop. That is consistent with ADR-003/008 ("no reuse of twenty-front React
code"; the desktop just hosts the same Leptos CSR bundle) and with wave 0's own framing
("port of `twenty-companion` (Electron desktop app) … as a Tauri app",
`research/frontend-inventory.md:632-633`). Recorded as **ADR-014** (§8).

Shell design (crate `crates/desktop`, **Tauri 2.12.7** — the current stable v2 line, verified):
- **Rust main process** = the Tauri host; the Leptos bundle is served by Tauri's bundled
  asset protocol (`tauri.conf.json` `frontendDist` = the trunk `--release` output) OR proxied
  to `front-serve` for a self-hosted server URL (ADR-008: "URL configurable",
  `rust/DECISIONS.md:14`; ADR-002 "Tauri does not embed the backend", `rust/DECISIONS.md:15`).
  Default: point the webview at the configured `SERVER_BASE_URL`, exactly as the browser does,
  so the desktop app is a thin native shell over the same web app (keeps parity trivial).
- Capabilities: window management, tray (the companion's tray-menu/calendar-agenda idea,
  `frontend-inventory.md:632`), autostart, deep-link registration (`twenty://…` → `AppPath`),
  notifications for AI/workflow runs, and the tier-1 `gloo-worker` + COOP/COEP setup (§5.1)
  enabled *here only* because the webview is same-origin with the API when self-hosting.
- **Cargo-tauri** CLI is the build step (`rust/MAP.md:14` lists it as not-yet-installed —
  wave-5 prerequisite). Windows/macOS/Linux per `README.md:80,139-143`.

### 5.6 Threading summary table

| Work | Thread | Mechanism | Gate |
|---|---|---|---|
| SSE parse + fan-out | worker | gloo-worker 0.7 scoped task | burst of 200 events, 0 dropped frames |
| GraphQL response normalize (large) | worker | gloo-worker | 100-field page parse off main thread |
| Table scroll / slot reassignment | main | Leptos effect + rAF | 10k rows ≥55fps |
| Cell render (fixed 32px rows) | main | memoized keyed pool | <16ms/frame |
| Shared-memory rayon | (gated) | feature, desktop-only | off by default; COOP/COEP |
| Static asset delivery | server | tower-http 0.7 brotli + h2 | brotli on wasm/js; HTTP/2 ALPN |

---

## 6. Parity verification plan

**Two live apps compared in the same browser session.** Reference: `twenty-ref` at
`http://127.0.0.1:3001` (real Twenty 2.61.0, seeded activated workspace `00162a10`,
`rust/MAP.md:16-22`; recreation procedure `rust/MAP.md:17-22` + `rust/HANDOFF.md:64-68`).
Nudge front: the Leptos bundle served by `front-serve` (default `http://127.0.0.1:8080`,
`§4.3`) pointed at a Nudge server, **or**, during wave-5 development before wave-4 record APIs
land, pointed at the `twenty-ref` server (same `/metadata`+`/graphql` contract, §3.1) so
front-end parity can be proven independently of backend completeness. Document both modes.

**Tooling (available in this project).** ZCode browser automation drives both pages. Load the
`browser-use:control-browser` skill for navigation/screenshot/inspection and
`browser-use:web-gui-tester` for the black-box flow scripts. Run the reference first (the
`twenty-ref` container must be up and seeded per `rust/HANDOFF.md:64-68`) and the Nudge bundle
via `trunk serve` or `cargo run -p nudge-front-serve`. Screenshot evidence is captured with the
browser tool's screenshot into `rust/parity/` (new dir, wave-5-owned) and compared side by side.
This extends the existing Rust-side parity harness (`python3 -I tools/parity-compare.py` for
API-shape diffs, `tools/parity-bench.py` for throughput, `rust/HANDOFF.md:69-75`) with a
**visual** layer for wave 5.

**Named flows (mirror `rust/MAP.md:33` wave scope + the user's list), each a scenario in
`rust/tools/e2e/*.inc.sh` style with a screenshot pair and DOM assertions:**

1. **Sign up** — `/welcome`: email → password → (2FA if enabled) → login-token exchange;
   assert the auth state atoms populate (`currentUserState`, §1.5) and the app transitions to
   the workspace router (§1.1). Cookie `__Host-twenty-session` set (wave 2 contract,
   `rust/MAP.md:82-86`).
2. **Create workspace / activation** — `/workspace-activation` then `/create/profile`
   (`AppPath.ts:11-12`); hits `signUpInNewWorkspace`/`activateWorkspace` REST
   (`rust/MAP.md:145-155`).
3. **People index with filters/sort/pagination** — `/objects/people`: table renders 32px rows,
   metadata-driven columns (§1.3); apply a TEXT filter (operand `CONTAINS`,
   `filterOperandsMap.ts`), a sort, page past 240 slots; assert generated
   `FindManyPeople` query shape (§3.2) matches reference on the wire (`tools/parity-compare.py`
   request replay).
4. **Open record** — click a row → `/object/person/:id` (§1.2); show page loads the record via
   the same `QueryCache`/`RecordIndex` (§3.3).
5. **Create company** — command-menu / `+` → `CreateOneCompany` (§3.2); optimistic row appears
   before SSE confirm (§3.5).
6. **Relate person ↔ company** — set a RELATION field; `triggerAttachRelationOptimisticEffect`
   port patches both cached lists (§3.5).
7. **Search** — Cmd-K → `Search` query (§3.2) returns cross-object `edges` with labels.
8. **Settings object create** — `/settings/objects/new` (`SettingsPath.NewObject` :33) →
   `CreateOneObjectMetadataItem` (§3.2) → metadata store hash updates → new object appears in
   the drawer and `/objects/<newPlural>` resolves.
9. **Dark mode** — toggle sets `.dark` on `<html>` (`ThemeProvider.tsx:14-19`); screenshot each
   core screen in light and dark and check the token table (§2.3) via computed styles.

**Screenshot evidence procedure.** For each flow: (a) capture the reference at
`:3001/<path>` and Nudge at `:8080/<path>` at the same viewport (1280×800 desktop; 390×844 for
mobile `/home`); (b) store PNG pairs under `rust/parity/<flow>/`; (c) record computed-style
deltas for the token anchors in §2.3 (background/font/border/accent) and the layout metrics in
§2.4 (header 48, side panel 400 default, drawer 220 default, row 32). A visual diff threshold
is set per flow (default: SSIM ≥ 0.97 for static screens; exact token match for colors, since
those are deterministic).

**Pass/fail ledger format** (append a `wave5/frontend` section to `rust/FEATURE-PARITY.md`,
the tracked feature status file `README.md:178-181`). One row per named flow, format:

```
| flow | screen(s) | assertion | light | dark | tokens | result | evidence |
|------|-----------|-----------|-------|------|--------|--------|----------|
| people-index | /objects/people | 32px rows, generated FindManyPeople | PASS | PASS | 0 deltas | PASS | rust/parity/people-index/*.png |
```

`result` = `PASS` only when light+dark+token-diff all pass; otherwise `FAIL(<reason>)`. Deviations
that are deliberate get their own `DEVIATION:` cell citing the ADR (§8), exactly as the auth
wave records "Deviations" (`rust/MAP.md:172-178`) and gaps (`rust/HANDOFF.md:108-117`).

---

## 7. Units U1..U6 (exclusive file ownership; new crates only)

Ownership is disjoint by crate/submodule so parallel waves never collide with backend waves
(backend owns `crates/core`, `crates/server`; none of these units touch those, per the task
constraint). Claims recorded in `rust/MAP.md` Units section (`rust/MAP.md:108-110`) on merge.

| Unit | Deliverable | Owned files (exclusive) | Needs backend wave | Acceptance criteria |
|---|---|---|---|---|
| **U1** front-ui | Design tokens + component primitives + icon set as a runnable component-gallery page | `crates/front-ui/**` (src, `theme/generated/*.css`, icons, `design-tokens` port) | none | gallery renders all §2.5 components; light/dark token sheet byte-compares against `theme-{light,dark}.css` anchors in §2.3/§2.4 (0 numeric deltas); `cargo clippy -D warnings` clean |
| **U2** front-core | GraphQL client (2 endpoints), metadata store, `QueryCache`+`RecordIndex`, SSE decoder, optimistic engine, generated-query builder | `crates/front-core/**` | **wave 3** (metadata store content) + **wave 4** (`/metadata`, `/graphql`, SSE, `AddQueryToEventStream`) for integration; dev-testable against `twenty-ref` :3001 immediately | metadata store hydrates from `FindMinimalMetadata`+stale-load, hash-diff reload on a metadata event; a `FindMany` round-trip parses+indexes; an `OnEventSubscription` frame patches the right `queryIds`; optimistic edit→confirm→rollback unit-tested (native, no browser) |
| **U3** front-app shell + auth | `leptos_router` mirroring §4.6, root/workspace split, auth/onboarding pages, layouts (drawer/header/side-panel surfaces), theme toggle, `MinimalMetadataGate`, session cookie handling | `crates/front-app/src/{app,auth,onboarding,routing,layout,client_config,main.rs,index.html,Trunk.toml}/**` | **wave 2** (auth — done, `rust/MAP.md:76-100`) + U2 | flows 1,2,9 (§6) PASS against `twenty-ref`; deep links resolve to the same URL as reference; unauth→`/welcome`, auth→workspace router transition verified |
| **U4** front-app record surfaces | Record index (TABLE/KANBAN/CALENDAR/LIST dispatch §1.3), 240-slot virtualizer (§5.2), inline field edit + display/input meta-types (§1.3), record show page, views/filters/sort/group, view bar, search | `crates/front-app/src/object_record/**`, `.../views/**`, `.../search/**`, `.../command_menu/**`, `.../side_panel/**` | **wave 4** (record query layer, generated ops) — hard requirement; dev against `twenty-ref` | flows 3,4,5,6,7 (§6) PASS; 10k-row scroll ≥55fps / <100ms (budget §5.2); generated `FindMany`/`CreateOne`/relation ops wire-match reference |
| **U5** front-app settings | Settings tree + data-model editors (objects/fields/indexes/validation), members/roles matrix, API keys, general/profile/experience | `crates/front-app/src/settings/**`, `.../navigation/**`, `.../navigation_menu_item/**` | **wave 3** (metadata mutations) + **wave 4** | flow 8 (§6) PASS: create object → appears in drawer + `/objects/<new>`; edit field → metadata hash reload; API key create/revoke parity; permission gating on settings routes (§1.4) verified |
| **U6** desktop + static serving + threading | `crates/desktop` Tauri 2 shell (§5.5), `crates/front-serve` HTTP/2+brotli+SPA catch-all (§5.4), tier-1 gloo-worker wiring (§5.1) | `crates/desktop/**`, `crates/front-serve/**`, `crates/front-core/src/workers/**` | **wave 4** (to serve a complete app); front-serve testable standalone | `cargo tauri build` produces a runnable app on macOS; webview loads the bundle against a configured `SERVER_BASE_URL`; `curl -I` on `front-serve` shows `HTTP/2` + `content-encoding: br` for `.wasm`; SSE decode runs off the main thread (no dropped frames on event burst §5.6) |

**Dependency order:** U1 → U2 → U3 → {U4, U5} (parallel) → U6. U1 and U2 can start immediately
independent of all backend waves (U2 integrates against `twenty-ref`). U3 needs auth (wave 2,
done) + U2. U4 is the record surfaces and is the one unit **strictly gated on wave 4's record
query layer** for real integration (the whole `FindMany`/`CreateOne`/SSE-`AddQueryToEventStream`
family, §3.2/§3.4) — although every U4 flow is verifiable against the `twenty-ref` Node server
during development, so U4 *work* does not block on wave 4, only its final same-database parity
run does. U5 needs wave-3 metadata mutations (the action runner in `rust/MAP.md:236-243`,
U6/U7 of the migrations wave) for create/edit-object from the UI. U6 is last because it wraps
the assembled app.

**No file conflicts with backend waves:** every path above is under `crates/front-*` or
`crates/desktop`; backend waves own `crates/core` and `crates/server` exclusively. The only
shared surface is `Cargo.toml` workspace `members` (`Cargo.toml:3`) — additive, coordinate at
merge (each unit adds its own member line), and the existing reservation comment
(`Cargo.toml:4-5`) already anticipates it.

---

## 8. Scoping decisions & ADRs (wave 5)

Out-of-scope within the port for **wave 5** (deferred to later waves, mirroring how ADR-013
documented a deliberate gap `rust/DECISIONS.md:75-79` and how waves record "Deviations"
`rust/MAP.md:172-178`). These are the parts of the wave-0 40k-LOC MVP estimate
(`research/frontend-inventory.md:555-637`) that this wave does **not** cover:

- **Workflow diagram editor visual complexity** (`modules/workflow/workflow-diagram/` on
  `@xyflow/react`, `frontend-inventory.md:254-260`) — wave 5 ships the workflows **list and
  show pages** (routes `/workflows`, `/workflow/:id`, §1.2) only if U4/U5 time allows; the
  node/edge canvas, drag-to-connect, zoom/pan are **wave 7** (workflows, `rust/MAP.md:36`).
  Deviation ADR-017.
- **Email / calendar sync UI** (`modules/activities/emails`, `modules/inbox` full composer,
  IMAP/SMTP/CalDAV connection flows §1.4 `AppPreferences*`, `frontend-inventory.md:265,297-303`)
  — wave 7 (messaging + calendar sync, `rust/MAP.md:36`). Wave 5 may show the settings
  *placeholders* but not functional sync screens.
- **AI chat, agents, dashboards/page-layout editors** (`ai-chat` 605 files, `page-layout` 811
  files, `frontend-inventory.md:265-272,274-284`) — **wave 6/7**. Wave 5 leaves the chat/inbox
  routes routing to a stub so deep links don't 404.
- **Rich-text editors** (TipTap/BlockNote `advanced-text-editor` 123 files,
  `frontend-inventory.md:306`) — wave 6 (activities); wave 5 table cells show RICH_TEXT as
  read-only markdown.
- **Spreadsheet import, record export CSV, log console, marketplace, logic functions (Monaco),
  MCP servers, billing, SSO config UI, admin panel** (`frontend-inventory.md:305-311`) —
  waves 6-7.
- **Localization/Lingui catalogs** (35 locales, `frontend-inventory.md:548-549`) — wave 6;
  wave 5 renders English strings inline (parity screenshots are locale-stable by pinning `en`).

New ADRs to add to `rust/DECISIONS.md` when U3 lands (drafted here):

- **ADR-014 — Desktop shell is Tauri 2 (Nudge-original), not a port of upstream.** Context:
  upstream 2.61.0 has no Tauri and no `twenty-desktop`; its only desktop app is
  Electron-based `twenty-companion` (`twenty-companion/package.json:49`). Decision: the
  committed Tauri 2 shell (ADR-008) is kept as a Nudge-original native wrapper hosting the
  Leptos CSR bundle against a configurable server URL; `twenty-companion`'s Electron-specific
  behavior (call recording, RecallAI SDK) is out of wave-5 scope. Consequences: desktop parity
  is measured against the *web* app, not the companion; cargo-tauri becomes a wave-5 toolchain
  prereq.
- **ADR-015 — Cache is query-level with a normalized id side-index, not Apollo-normalized.**
  Rationale §3.3; consequences §3.3–3.5.
- **ADR-016 — CSR, HTTP/2+brotli static delivery, and multithreading are improvements over a
  single-threaded SPA reference.** Context: upstream has no workers/SSR/preconnect (§5, §4.4–4.5).
  Decision: keep CSR (ADR-008), add preconnect + metadata prefetch (§4.5), add brotli/HTTP-2 +
  SPA catch-all in a dedicated `front-serve` crate (§5.4), and offload SSE/normalize to wasm
  workers with shared-memory rayon gated off (§5.1). Consequences: `front-serve` and
  `crates/desktop` are new, backend-independent crates.
- **ADR-017 — Wave-5 feature scope is the daily-driver surface only.** Context: 12,700-file
  reference (`rust/DECISIONS.md:45` ADR-008). Decision: wave 5 = auth +
  metadata + record index (all four view types) + record show + views/filters/sort/group +
  command menu + side panel + navigation/favorites + settings (data-model, members/roles,
  general, API keys) — matching `rust/MAP.md:33`; workflows/AI/email/dashboards/rich-text/
  import/billing deferred to waves 6-7 as listed above. Consequences: wave 6/7 own the
  remaining surfaces; wave-5 routes for deferred features render gated stubs.

---

## Known gaps / open questions

1. **Leptos 0.8 vs 0.9.** 0.9 is rc-only at writing (`crates.io` lists 0.9.0-rc1, 0.8.23 last
   stable). Wave 5 pins **0.8.23**; if 0.9 stable lands during the wave, re-evaluate the router
   and worker APIs but do not adopt an rc for a product wave (rule: every wave builds/tests/runs,
   `README.md:161-162`).
2. **Multi-module wasm for route splitting.** §4.5 default may collapse to a single bundle if
   dynamic wasm modules regress startup on the pinned trunk/wasm-bindgen; the fallback and its
   ADR note are stated, but the decision is made in U3 after measuring first-paint.
3. **`/metadata` vs `/graphql` client-name inversion.** Documented (§3.1). The wave-4 server
   must expose the subscription transport on **`/metadata`** (per `SSEClientEffect.tsx:62`),
   which contradicts the older note at `graphql-rest-api.md:166`; reconcile before U2
   integration — either wire the port's SSE to `/metadata` (current front behavior) or update
   the front to `/graphql` and fix the doc. Recommended: follow the front (`/metadata`).
4. **Collection-hash invalidation contract.** The metadata store diffs
   `currentCollectionHash`/`draftCollectionHash` (§1.5); the exact hash algorithm
   (order-insensitive) must match the server's `metadataVersion`/hash emitted by the wave-3
   runner (`rust/MAP.md:238-239`). Pin it in U2 against `twenty-ref`.
5. **Screenshot SSIM thresholds are provisional** (§6); calibrate on the first U4 run against
   the live reference and freeze them in `rust/FEATURE-PARITY.md`.

---

*End of wave-5 front-end architecture. No product code shipped in this unit; this file is the
implementable plan for U1–U6.*
