# UI parity

Reference: Twenty CRM `twenty/v2.61.0`. Measure both applications under the same
viewport, theme, fonts, device pixel ratio and authenticated state. Default visual
tolerance is zero; branding/text substitutions must be explicit.

## Icon and design-token audit

### Tabler catalog

- Upstream export set: 4,187 icons (`ALL_ICONS` in the 2.61.0 frontend bundle).
- Local source: `crates/front-ui/src/icons/generated.rs`.
- Generator: `crates/front-ui/tools/gen_icons.py`.
- Evidence: fresh regeneration is byte-identical before formatting; 4,186 icons have
  `data_twenty_icon: false`, and the custom `IconAddressBook` has `true`, matching
  upstream SVGR output.
- Defaults now match Tabler: size 24, stroke 2, `currentColor`; `className`, `style`
  and `aria-hidden` are forwarded.

### Custom SVG catalog

- Upstream assets: 46 SVG files under `packages/twenty-ui/src/assets/icons`.
- Local source: `crates/front-ui/src/icons/custom.rs`.
- Generator: `crates/front-ui/tools/gen_custom_icons.py`.
- Pin: commit `4bac2940dcdcbf4bb4c974b628cd5e1b0a2e37e0`; the generator fails if the
  upstream asset set differs from its manifest.
- Corrections in this slice:
  - custom assets are fetched from the pinned commit, not moving `main`;
  - every custom SVG receives the upstream `data-twenty-icon=""` root attribute;
  - `IconLockCustom` is separate from the Tabler `IconLock`;
  - default sizes follow `theme.icon.size` (`lg=20`, explicit 24 where upstream uses 24);
  - stroke and color behavior follows each upstream component;
  - illustrations use the upstream wrapper styles and `accent3`/`accent8` theme tokens;
  - `IconSparkle2`, `IconChartBarHorizontal` and `ThinkingOrbitLoaderIcon` match their
    upstream root attributes, defaults and animation markup.
- Unit evidence: 22 icon contract tests pass.
- Browser evidence (built-in browser, reference `:3001`, local gallery `:1420`):
  - `IconUser`, Tabler `IconLock` and custom `IconAddressBook` match the reference DOM
    after canonicalizing attribute order, whitespace and Leptos `<!---->` markers;
  - 40 custom components exported by the built reference icon chunk match local path
    geometry and computed width, height, color, fill, stroke and stroke-width exactly;
  - `IconSparkle2`, `IconChartBarHorizontal` and `ThinkingOrbitLoaderIcon` have canonical
    SVG equality with the reference; the chart wrapper's computed transform and flex
    behavior also match;
  - the five tree-shaken custom assets (`groq`, `illustration-many-to-many`,
    `illustration-one-to-one`, `lock`, `many-to-one`) are verified against the pinned
    source/raw SVG because the production bundle does not export those components.
- Status: `VERIFIED` for the icon catalog; component/page interaction parity remains open.

## Remaining UI units

### U1: production overlays and navigation

Files: `crates/front-ui/src/components/{dialog,popover,menu,tabs}.rs`.

- Dialog: close lifecycle, focus entry/trap/restoration, Escape and scroll locking.
- Popover: outside dismissal, focus return, collision/placement and portal bounds.
- Menu: real buttons, arrow/Home/End/typeahead, selection and disabled states.
- Tabs: activation callback, keyboard model and tab-to-panel ARIA linkage.
- Migrate create-dialog and view-picker markup to the shared primitives.

### U2: shared metadata/context substrate

Files: `crates/front-core/src/{projection,metadata_store,query_builder}.rs`.

- Add object/field IDs and label identifier metadata.
- Add typed command-menu and settings permission/feature-gate models.
- Do not reuse old title/favicon captures until authenticated final URLs are asserted.

### U4: record index/show

Files: `crates/front-app/src/{record_index,record_views,pages,record_order}.rs`.

- Metadata-driven columns, selection, column sorting, named views, filters, groups,
  aggregates and drag/drop.
- Generic per-type display/editors; remove static People/Email/Job/Company assumptions.
- Preserve the 240-slot virtual window and prove no more than 241 rows render.

### Shell: contextual command menu and routed panel

Files: `crates/front-app/src/{command_menu,side_panel}.rs`.

- Current object/record/selection context, sections, permissions and hotkeys.
- Panel deep links, history stack, back/forward, close/Escape, focus restoration,
  resize bounds and motion.

### U5: settings leaves

File: `crates/front-app/src/pages.rs`.

Replace the seven placeholder links with Profile, Experience, Data Model,
Members/Roles, API Keys and General editors, including permission gates and real
metadata refresh behavior.

## Evidence that must not be reused

- Password/sign-in/toast ledgers are marked legacy in FEATURE-PARITY.md.
- `parity/ui/welcome-diff.json` contains 157 differences despite older prose saying 12.
- `head-command-menu-reference.json` and `head-record-show-reference.json` landed on
  `/welcome`; they do not certify those product surfaces.
- `parity/people-index/` and `parity/record-show/` contain only Nudge PNGs.
- Existing UI JSON omits candidate commit/build, fixture and authenticated state.
