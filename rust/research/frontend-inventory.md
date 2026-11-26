# Twenty CRM — Front-end Inventory (for the Leptos/Tauri port)

Source (read-only): `C:\Users\tonminh123\Documents\crm-research\twenty`
Scope: `packages/twenty-front/src/**`, `packages/twenty-ui/**`, `packages/twenty-shared/**`,
`packages/twenty-emails/**`, `packages/twenty-companion/**`.
Target: Leptos (Rust, CSR via trunk) in Tauri 2; axum + async-graphql server (ADR-002..005 in
`rust/DECISIONS.md`). Sizes below exclude `__tests__`, `__stories__`, `*.test.*`, `*.stories.*`.
Canonical design guide to read alongside this file: repo-root `DESIGN.md` (281 lines).

---

## 1. Route / page inventory (react-router v7, `createBrowserRouter`)

### 1.1 Router files
- `modules/app/hooks/useCreateRootAppRouter.tsx` — pre-workspace router (sign-in, invite, verify, reset, deep-link 404).
- `modules/app/hooks/useCreateWorkspaceAppRouter.tsx` — authenticated router (workspace layout, side panel, onboarding, settings, records).
- `modules/app/routing/utils/createWorkspaceRouteObjects.tsx` — workspace route objects (record index/show, workflows, agents, chat, page layouts).
- `modules/app/components/SettingsRoutes.tsx` — 1406-line settings tree with permission + feature-flag gating.
- `modules/app/routing/utils/getWorkspaceRouteObjectsForSurface.ts` — splits routes between `main` and `side-panel` surfaces; `routing/types/WorkspaceRouteObject.ts` carries `handle.workspaceSurfaces`.
- Layout shells: `modules/app/components/{DefaultLayout,MainAppLayoutWithSidePanel,BlankLayout}.tsx`, `modules/ui/layout/page/components/AuthFlowLayout.tsx`, `modules/onboarding/components/Onboarding{Step,Transition,Activation}Outlet.tsx`.

### 1.2 `AppPath` enum (`packages/twenty-shared/src/types/AppPath.ts`) → page component
- Auth/onboarding: `/welcome` → `pages/auth/SignInUp.tsx`; `/invite/:workspaceInviteHash` → same; `/verify` → `pages/onboarding/Verify.tsx`; `/verify-email` → `modules/auth/components/VerifyEmail.tsx`; `/reset-password/:token` → `pages/auth/PasswordReset.tsx`; `/authorize` → `pages/auth/Authorize.tsx`.
- Onboarding steps: `/workspace-activation` (`WorkspaceActivation.tsx`), `/create/profile` (`CreateProfile.tsx`), `/sync/emails` (`SyncEmails.tsx`), `/invite-team` (`InviteTeam.tsx`), `/book-call` (`BookCall.tsx`), `/plan-required` (`ChooseYourPlan.tsx`), `/plan-required/payment-success` (`PaymentSuccess.tsx`).
- Product: `/` → `pages/object-record/RecordIndexPage.tsx`; `/objects/:objectNamePlural` → same (table/kanban/calendar/list); `/object/:objectNameSingular/:objectRecordId` → `RecordShowPage.tsx`; `/page/:pageLayoutId` → `pages/page-layout/StandalonePageLayoutPage.tsx`; `/getting-started` → `pages/getting-started/GettingStartedPage.tsx`; `/home` → `pages/mobile-home/MobileHomePage.tsx` (mobile nav).
- Engines: `/workflows` → `pages/object-core/WorkflowCoreIndexPage.tsx`; `/workflow/:coreWorkflowId` → `WorkflowCoreShowPage.tsx`; `/agents` → `AgentCoreIndexPage.tsx`; `/agent/:agentId` → `AgentCoreShowPage.tsx`.
- AI: `/chat/:threadId?` → `pages/legacy-ai-chat/LegacyAiChatPage.tsx` → `pages/ai-chat/*`; `/inbox/:threadId?` → `pages/inbox/InboxPage.tsx`.
- Other: `/settings/*` (SettingsRoutes), `/dpa` (null element, deep link to DPA generator), `*` → `pages/not-found/NotFound.tsx`; root router `*` → `modules/app/components/RootDeepLinkRedirect.tsx`.
- Redirects: `/objects/workflows` → `/workflows` (`NavigatePreservingSearchAndHash`); `/object/agentChatThread/:id` → chat page (`getAiChatPageRoute`).

### 1.3 Side-panel surfaces (in-app, not URL routes)
`packages/twenty-shared/src/types/SidePanelPages.ts` — 57 page kinds in `modules/side-panel/pages/*`
(router `modules/side-panel/components/SidePanelRouter.tsx`, map in `constants/SidePanelPagesConfig.tsx`):
command-menu-display; merge-records; merge-with-record; update-records; edit-rich-text;
workflow-trigger-select-type; workflow-step-create/edit-type/edit/view/run-step-view/settings;
search-records; search-ai-chats; ask-ai; page-layout-dashboard-widget-type-select;
page-layout-tab-settings; page-layout-widget-settings; dashboard-chart-settings;
dashboard-iframe-settings; dashboard-record-table-settings; record-page-fields-settings;
record-page-field-settings; view-front-component; command-menu-edit;
page-layout-record-page-widget-type-select; compose-email; compose-calendar-event; create-related-record;
record-creation-form(+settings); record-update-form; send-campaign(+test); add-to-message-list;
workflow-versions; workflow-version; email-block-settings; email-page-style; email-block-style;
settings-metadata-translations; routed-page; workflow-core-filters; agent-core-filters;
agent-core-trigger-type; agent-core-trigger; agent-core-data-access; agent-core-tools; share-record;
snooze-ai-chat; assign-ai-chat.
Size 485 files / 27 809 LOC. Width `SIDE_PANEL_CONSTRAINTS = {min:320,max:600,default:400}`.

### 1.4 Settings routes (`SettingsPath` enum, `packages/twenty-shared/src/types/SettingsPath.ts`)
Mounted at `/settings/*`; gated by `settingsPermission` (`PermissionFlagType`) and
`requiredFeatureFlag` (`FeatureFlagKey`); some paths are hash-routed.
- Profile `profile`, `profile/two-factor-authentication/:strategy`; Experience `experience`.
- App preferences `app-preferences`, `app-preferences/:applicationId`, `app-preferences/accounts/:id`,
  `.../new`, `.../configuration`, `.../new-imap-smtp-caldav-connection`, `.../edit-imap-smtp-caldav-connection`.
- Workspace `general`, `general/subdomain`, `general/custom-domain`, `partner`, `enterprise`, `community`,
  `members`, `members/:workspaceMemberId`; Roles `members/roles`, `members/roles/create`,
  `members/roles/:roleId`, `members/roles/:roleId/object/:objectMetadataId`.
- Data model `objects`, `objects/overview`, `objects/:objectNamePlural` (+`new-field/select`,
  `new-field/configure`, `new-index`, `validation-rules/:id`, `:fieldName`, `object-translations`,
  `object-search`, `object-indexes`, `:fieldName/translations`), `objects/new`; Layout `layout`.
- Communications `communications`, `communications/unsubscribe`, `communications/blocklist`,
  `email/email-group/:messageChannelId`, `email/new-email-group`, `email/new-unsubscribe-topic`,
  `email/unsubscribe-topic/:id`; Billing `billing`, `billing/plans`, `billing/usage`,
  `billing/usage/user/:userWorkspaceId`, `billing/limits`, `billing/limits/new`, `billing/limits/:id/edit`.
- AI `ai`, `ai/prompts`, `ai/new-skill`, `ai/skills/:skillId`, `ai/tools/:toolIdentifier`,
  `ai/usage/user/:userWorkspaceId`, `ai/new-mcp-server`, `ai/mcp-servers/:mcpServerId`.
- Developers `developers`, `developers/apis/new`, `developers/apis/:apiKeyId`, `developers/webhooks/new`,
  `developers/webhooks/:webhookId`, `playground/graphql/:schema`, `playground/rest/:schema`.
- Applications `applications` + `applications/:applicationId/...` (connections, public-domain,
  logicFunctions, frontComponents, commandMenuItems, timelineActivityTypes, views, pageLayouts, content).
- Security `general#security`, `security/sso/new`, `security/approved-access-domain/new`;
  Legal `legal`, `legal/dpa`, `legal/dpa/new`; Admin `admin-panel` + health-status, users, workspaces,
  ai providers/models, config variables, application registrations, chat threads.
Settings pages `pages/settings/**` = 216 files / 24 922 LOC; `modules/settings/**` = 1509 files / 86 149 LOC.

---

## 2. Feature-module inventory (`packages/twenty-front/src/modules/**`)

### 2.0 Size map (files / LOC)
- Large: `object-record` 1855/101 237 · `settings` 1509/86 149 · `page-layout` 811/41 172 ·
  `workflow` 548/35 509 · `ai-chat` 605/27 714 · `side-panel` 485/27 809 · `ui` 618/26 101 (front-local layer).
- Medium: `activities` 362/19 512 · `object-core` 301/11 633 · `navigation-menu-item` 214/10 959 ·
  `command-menu-item` 201/9 483 · `views` 239/11 187 · `onboarding` 167/8 664 · `auth` 149/6 711 ·
  `inbox` 140/6 876 · `advanced-text-editor` 123/8 017 · `spreadsheet-import` 94/5 235 ·
  `log-console` 87/4 785 · `getting-started` 85/4 486 · `ai` 86/2 643 · `object-metadata` 155/5 781 ·
  `validation-rules` 76/3 944 · `navigation` 67/3 282 · `metadata-store` 54/1 899 · `apollo` 31/3 047 ·
  `sse-db-event` 46/2 207 · `client-config` 45/813 · `front-components` 39/2 322 · `marketplace` 31/1 478 ·
  `applications` 46/1 567 · `command-menu` 17/988 · `logic-functions` 25/1 043 · `related-record-paths` 18/688 ·
  `workspace*` 52/1 627 · `mcp-servers` 8/105 · `dashboards` 6/141 · `record-export` 3/135.
- Small/misc: `media-access`, `mention`, `geo-map`, `captcha`, `keyboard-shortcut-menu`,
  `information-banner`, `layout-customization`, `file`/`file-upload`, `sign-in-background-mock`.

### 2.1 Object record: table, kanban, calendar, list, show page
Sub-packages of `modules/object-record/`: `record-table` 337 (virtualized table, header/cell/row/body/section,
DnD, groups, aggregates) · `record-field` 418 (per-type display+input, §4.4) · `record-board` 105 (kanban) ·
`record-calendar` 45 · `record-index` 87 (container, view bar, header, empty states) · `record-group` 75
(group tabs, aggregates, visibility) · `record-picker` 52 · `record-filter` 37 · `object-filter-dropdown` 62 ·
`advanced-filter` 57 (AND/OR groups) · `record-show` 26 · `record-list` 28 · `record-field-list` 24 ·
`record-drag` 24 · `record-sharing` 32 · `record-form` 40 · `record-selection` 16 · `record-store`+`cache` 43 ·
title/inline/column-lookup cells 45 · `record-merge` 18 · `record-update-multiple` 7 · `record-aggregate` 5 ·
options/sort dropdowns 47 · `graphql` 22 (dynamic query generation).
- **Table**: `RECORD_TABLE_ROW_HEIGHT=32`, `RECORD_TABLE_COLUMN_MIN_WIDTH=104`, checkbox col `28`,
  DnD handle `12`, add-column button `32` (`record-table/constants/`); virtualization in
  `record-table/virtualization/`; cells `record-table-cell/components/RecordTableCell.tsx` +
  `RecordTableCell{Display,Edit}Mode.tsx`, hover/focus portals, arrow-key + hotkey effects; rows
  `record-table-row/components/` (`RecordTableRow`, `RecordTableFieldsCells`, `RecordTableDraggableTr`,
  sortable variants); header `record-table-header/` (resize handle, hidden-column dropdown, aggregates);
  groups `RecordTableRecordGroupRows` + section headers.
- **Kanban**: `record-board/components/RecordBoard.tsx`, `RecordBoardColumns`, `record-board-column/`,
  `record-board-card/` (+ anchored portal), `record-board-dnd/`, aggregate dropdown, column-width effect,
  sticky header, drag-select.
- **Calendar**: `record-calendar/components/RecordCalendar.tsx`, `RecordCalendarContainer`,
  `RecordCalendarDragDropContext`, `RecordIndexCalendarDataLoaderEffect`; layout `ViewCalendarLayout` =
  DAY|WEEK|MONTH (`packages/twenty-shared/src/types/ViewCalendarLayout.ts`); fields
  `view.calendarFieldMetadataId` / `calendarEndFieldMetadataId`.
- **List**: `record-list/` (ViewType LIST).
- **Show page**: `pages/object-record/RecordShowPage{Content,Header,Shell,Title}.tsx` +
  `record-show/` (`RecordShowResourceLoader`, `ObjectRecordShowPageBreadcrumb`,
  `PageLayoutRecordPageRenderer`) + `record-field-list/` (sections); `useRecordShowPage*` hooks load
  record + permissions; sections come from the RECORD_PAGE page layout.
- **Dispatch** (`record-index/components/RecordIndexContainer.tsx`): TABLE → `RecordIndexTableContainer`,
  KANBAN → `RecordBoardContainer`, CALENDAR → `RecordIndexCalendarContainer`, LIST → `RecordList`.
  `ViewType` (`packages/twenty-shared/src/types/ViewType.ts`) = TABLE, KANBAN, CALENDAR, LIST,
  FIELDS_WIDGET, TABLE_WIDGET, KANBAN_WIDGET, LIST_WIDGET, CALENDAR_WIDGET.
- **Data (GraphQL)**: `FindMany<Objects>` is *generated at runtime* by
  `object-record/utils/generateFindManyRecordsQuery.ts` (vars `$filter: XFilterInput`,
  `$orderBy: [XOrderByInput]`, `$lastCursor`, `$limit`, `$offset`; returns `edges{node,cursor}`,
  `pageInfo{hasNextPage,hasPreviousPage,startCursor,endCursor}`, `totalCount`); field selection via
  `object-metadata/utils/mapObjectMetadataToGraphQLQuery.ts` + `mapFieldMetadataToGraphQLQuery.ts`
  (composite → sub-fields, relations → target join columns, morph → per-target fields).
  Also `FindOne<X>`, `CreateOne<X>`, `UpdateOne<X>`, `DeleteOne<X>` (soft), `DestroyOne<X>`,
  `CreateMany<X>`, `UpdateMany<X>`, `DeleteMany<X>`, `DestroyMany<X>`, `RestoreMany<X>`, `MergeMany<X>`
  (generators in `object-metadata/utils/generate{Create,Update}OneRecordMutation.ts`);
  `Aggregate<X>` (`useAggregateRecords`), `GroupBy<X>` (`useGroupByRecordsQuery`), `Search`,
  `FindDuplicateRecords`, `ObjectRecordCounts`, `GetRecordPermissions`, `GetRecordSharing`,
  `SetRecordShare`, `RemoveRecordShare`, `SetRecordPrivate`, `ExportRecords`; live updates via
  `AddQueryToEventStream` / `RemoveQueryFromEventStream` + `subscription OnEventSubscription`.

### 2.2 Views, filters, sort, group-by
`modules/views/` (239) + `record-filter/`, `object-filter-dropdown/`, `advanced-filter/`,
`record-sort/`, `record-group/`. Fragments in `modules/views/graphql/fragments/`:
- `ViewFragment`: id, name, objectMetadataId, type, key, icon, position, isCompact,
  kanbanAggregateOperation(+FieldMetadataId), mainGroupByFieldMetadataId, shouldHideEmptyGroups,
  shouldDisplayGroupsAsTabs, kanbanColumnWidth, groupLoadLimit, anyFieldFilterValue,
  calendarFieldMetadataId, calendarEndFieldMetadataId, calendarLayout, visibility, universalIdentifier,
  isActive + viewFields, viewFieldGroups, viewFilters, viewFilterGroups, viewSorts, viewGroups.
- `ViewField`: id, fieldMetadataId, isVisible, isCanonical, relationTargetFieldMetadataId, position, size,
  aggregateOperation, viewFieldGroupId. `ViewFilter`: id, fieldMetadataId, operand, value,
  viewFilterGroupId, positionInViewFilterGroup, subFieldName, relationTargetFieldMetadataId.
  `ViewSort`: id, fieldMetadataId, direction, subFieldName, relationTargetFieldMetadataId.
  `ViewGroup`: id, isVisible, fieldValue, position.
- **Operands** (`packages/twenty-shared/src/utils/filter/utils/filterOperandsMap.ts`): TEXT/EMAILS/
  FULL_NAME/ADDRESS/LINKS/PHONES/RAW_JSON/FILES/MULTI_SELECT/ACTOR/ARRAY → CONTAINS,
  DOES_NOT_CONTAIN, IS_EMPTY, IS_NOT_EMPTY; NUMBER/RATING → IS, IS_NOT, ≥, ≤, empty; CURRENCY → ≥, ≤,
  empty; DATE/DATE_TIME → IS, IS_RELATIVE, IS_IN_PAST, IS_IN_FUTURE, IS_TODAY, IS_BEFORE, IS_AFTER,
  empty; RELATION → IS, IS_NOT, empty; SELECT → IS, IS_NOT, empty; BOOLEAN → IS; TS_VECTOR →
  VECTOR_SEARCH; composite sub-fields in `compositeFieldFilterOperandsMap.ts` (CURRENCY.amountMicros vs
  currencyCode; ACTOR.source/workspaceMemberId → IS/IS_NOT/empty). Enum:
  `packages/twenty-shared/src/types/ViewFilterOperand.ts`.
- **Sort**: `ViewSort.direction` + `subFieldName`; helpers `object-record/utils/reverseOrderBy.ts`,
  `extractOrderByFieldNames.ts`. **Group-by**: `record-group/` tabs + counts
  (`RecordGroupTabsRecordCountsEffect`), aggregate dropdown, visibility, reorder; allowed field types per
  `packages/twenty-shared/src/utils/fieldMetadata/isFieldMetadataSupportedInGroupBy.ts`.
- **Ops**: FindManyViews, FindAllViews, FindTableWidgetViews, FindFieldsWidgetViews, CreateView,
  UpdateView, DestroyView, CreateManyViewFields, UpdateViewField, DestroyViewField, CreateViewFilter,
  UpdateViewFilter, DestroyViewFilter, CreateViewFilterGroup, UpdateViewFilterGroup,
  DestroyViewFilterGroup, CreateViewSort, UpdateViewSort, DestroyViewSort, CreateManyViewGroups,
  UpdateManyViewGroups.

### 2.3 Command menu (Cmd-K)
`modules/command-menu/` (17) + `modules/command-menu-item/` (201/9 483).
- Item model (`CommandMenuItemFields`): id, universalIdentifier, applicationId, coreWorkflowVersionId,
  workflowVersionId, frontComponentId(+name,isHeadless), engineComponentKey, label, icon, shortLabel,
  position, isPinned, payload(`PathCommandMenuItemPayload.path`), hotKeys,
  conditionalAvailabilityExpression, conditionalPinnedExpression, availabilityType,
  availabilityObjectMetadataId, availabilityCoreObjectName, navigationTargetObjectMetadataId,
  pageLayoutId, isActive.
- Availability (`packages/twenty-shared/src/types/CommandMenuItemAvailabilityType.ts`): GLOBAL,
  GLOBAL_OBJECT_CONTEXT, RECORD_SELECTION, FALLBACK.
- Engine commands `command-menu-item/engine-command/`: global (ComposeCampaign, ComposeEmail, EditLayout,
  NewAiChat); record (AddToFavorites, RemoveFromFavorites, DeleteRecords, DestroyRecords, RestoreRecords,
  ExportRecords, ShareRecord, MergeWithRecord, UpdateMultipleRecords, TriggerWorkflowVersion,
  AddRecordsToMessageList); no-selection (CreateNewRecord, CreateNewView, ImportRecords,
  SeeDeletedRecords, HideDeletedRecords, CreateNewIndexRecord); single-record (NavigateToNext/PreviousRecord).
- `EngineComponentKey` (server enum; see `packages/twenty-client-sdk/src/metadata/generated/schema.graphql`):
  CREATE_NEW_RECORD/VIEW, DELETE/DESTROY/RESTORE_RECORDS(+MULTIPLE/SINGLE), MERGE_*, NAVIGATE_TO_NEXT/
  PREVIOUS_RECORD, GO_TO_{PEOPLE,COMPANIES,OPPORTUNITIES,NOTES,TASKS,WORKFLOWS,DASHBOARDS,RUNS,SETTINGS},
  SEARCH_RECORDS(+FALLBACK), ASK_AI, NEW_AI_CHAT, SHARE_RECORD, ADD_TO/REMOVE_FROM_FAVORITES,
  EXPORT_*, EDIT_DASHBOARD_LAYOUT, DUPLICATE_DASHBOARD, TEST_WORKFLOW, TRIGGER_WORKFLOW_VERSION,
  SEE_VERSIONS_WORKFLOW, ...
- Ops: FindManyCommandMenuItems, UpdateCommandMenuItem, ResetCommandMenuItem, DuplicateMessageList,
  TriggerAddPeopleToMessageListJob. Hotkeys: `modules/ui/utilities/hotkey/` (13) on `react-hotkeys-hook`;
  `modules/keyboard-shortcut-menu/`.

### 2.4 Side panel — 485 files / 27 809 LOC
Shell `SidePanelContainer`, `SidePanelTopBar` (height = `APP_HEADER_HEIGHT` 48), `SidePanelBackButton`,
`SidePanelExpandButton`, `SidePanelCloseButton`, `SidePanelRouter`, `SidePanelSubPageRouter`,
`SidePanelList`, `SidePanelGroup`; state `side-panel/states/` (navigation stack, width, page info,
search, morph items); routing `side-panel/routing/`.

### 2.5 Sidebar, favorites, navigation — `navigation/` 67 + `navigation-menu-item/` 214/10 959
Drawer: `navigation/components/AppNavigationDrawer.tsx`, `MainNavigationDrawer*`, collapsible sections,
`NavigationDrawerItem*`, workspaces switcher, mobile nav bar. Widths `NAVIGATION_DRAWER_CONSTRAINTS`
{min:180,max:350,default:220}, collapsed 48 (`modules/ui/navigation/navigation-drawer/constants/`).
Item types (`packages/twenty-shared/src/types/NavigationMenuItemType.ts`): VIEW, FOLDER, LINK, OBJECT,
RECORD, PAGE_LAYOUT, AGENT_CHAT_CHANNEL, NATIVE_PAGE, WORKFLOW, AGENT, TAB, SECTION.
**Favorites** = `NavigationMenuItemType.RECORD` rows created by
`command-menu-item/engine-command/record/components/{AddTo,RemoveFrom}FavoritesCommand.tsx` via
`createManyNavigationMenuItems` (position = next user favorite position); shown in the drawer Favorites section.
Ops: FindManyNavigationMenuItems, CreateManyNavigationMenuItems, UpdateManyNavigationMenuItems,
DeleteManyNavigationMenuItems. Fragment `NavigationMenuItemFields`: id, type, userWorkspaceId,
targetRecordId, targetObjectMetadataId, viewId, parentId, name, link, nativePageKey, icon, color,
pageLayoutId, agentChatChannelId, workflowId, agentId, position, applicationId.

### 2.6 Settings pages — `modules/settings/` + `pages/settings/**`
Areas: accounts (mail/calendar channels), admin-panel, ai (prompts/skills/tools/mcp), app-preferences,
application-registrations, applications, billing, communications, community, config-variables,
data-model (objects/fields/indexes/translations/validation rules), developers (API keys, webhooks,
REST+GraphQL playground), domains, emailing-domains, enterprise, event-logs, experience, lab, layout,
legal, logic-functions, mcp, mcp-servers, members, privacy, profile, roles, security, translations,
two-factor-authentication, unsubscribe-topics, unsubscribers, usage, workspace, workspace-member.
Key ops: CreateApiKey/GetApiKeys/RevokeApiKey, CreateWebhook/GetWebhooks/UpdateWebhook/DeleteWebhook;
CreateOneObjectMetadataItem, UpdateOneFieldMetadataItem, CreateOneIndexMetadataItem, CreateValidationRule;
MyConnectedAccounts, MyMessageChannels, MyCalendarChannels, MyMessageFolders, SaveImapSmtpCaldavAccount,
StartChannelSync; CreateOneRole, UpsertFieldPermissions, UpsertObjectPermissions,
UpsertRowLevelPermissionRules; CreateApprovedAccessDomain, CreateOIDCIdentityProvider,
CreateSAMLIdentityProvider; GetUsageAnalytics; MetadataTranslations.

### 2.7 Onboarding — `modules/onboarding/` 167/8 664 + `pages/onboarding/*` (10 pages, 1 559 LOC)
Flow: workspace activation → create profile → sync emails → invite team → book call → plan required.
Also `modules/getting-started/` (85 files): `GettingStartedProgress`, `PrepareGettingStartedWorkflow`,
`GettingStartedWorkflowPreviews`. Ops: EnrichWorkspaceCompany, CompleteBookCallOnboardingStep,
GoBackToPreviousOnboardingStep, SkipSyncEmailOnboardingStep, StartWorkspaceSetupChat, GetInviteSuggestions.

### 2.8 Auth — `modules/auth/` 149/6 711 + `pages/auth/*`
Steps (`SignInUpStep`, `modules/auth/states/signInUpStepState.ts`): init, email, password,
emailVerification, workspaceSelection, workspaceCreation, SsoIdentityProviderSelection,
TwoFactorAuthenticationVerification, TwoFactorAuthenticationProvision, TwoFactorAuthenticationRecovery.
Ops (`modules/auth/graphql/`): SignIn, SignUp, SignUpInNewWorkspace, SignUpInWorkspace, SignOut,
Impersonate, StopImpersonation, GetLoginTokenFromCredentials, getAuthTokensFromLoginToken,
getAuthTokensFromOTP, getAuthTokensFromSsoExchangeToken, getAuthTokensFromTwoFactorAuthenticationRecoveryCode,
GetAuthorizationUrlForSSO, VerifyEmailAndGetLoginToken, VerifyEmailAndGetWorkspaceAgnosticToken,
UpdatePasswordViaResetToken, EmailPasswordResetLink, ResendEmailVerificationToken, RenewToken,
GenerateApiKeyToken, GeneratePlaygroundToken, generateTransientToken, CheckUserExists,
CheckWorkspaceSubdomainAvailability, GetWorkspaceCreationDefaults, FindApplicationRegistrationByClientId,
GetPublicWorkspaceDataByDomain, authorizeApp.
Session = http-only cookie (`credentials:'include'` on the Apollo upload/REST links,
`modules/apollo/services/ApolloFactory.ts`); `REST_API_BASE_URL = ${SERVER_BASE_URL}/rest`
(`modules/apollo/constants/RestApiBaseUrl.ts`). `GetCurrentUser`
(`modules/users/graphql/queries/getCurrentUser.ts`, fragment `UserQueryFragment`) returns user +
workspace + `permissionFlags` + `objectRecordsPermissions` + feature flags + billing + AI config.

### 2.9 Workflows UI — `modules/workflow/` 548/35 509 + `modules/object-core/` 301
Diagram editor `workflow/workflow-diagram/` (nodes/edges on `@xyflow/react`), versions
`workflow/workflow-version/`, trigger `workflow/workflow-trigger/`, variables `workflow/workflow-variables/`.
Step types (`workflow/workflow-steps/workflow-steps/<type>-action/`): ai-agent, classify, code, delay,
filter, find-records, form, http-request, if-else, iterator, logic-function, mcp-tool, pick-record,
send-chat-message, wait-for-event.
Legacy ops: ComputeStepOutputSchema, RetryWorkflowRun, StopWorkflowRun, SubmitFormStep, TestHttpRequest,
TestMcpToolStep, TestWorkflowCodeStep, UpdateWorkflowRunStep, WorkflowCodeStepAvailablePackages.
Core ops: GetCoreWorkflows, GetCoreWorkflow, GetCoreWorkflowVersions, CreateCoreWorkflow,
CreateCoreWorkflowVersionStep, UpdateCoreWorkflowVersionStep, UpdateCoreWorkflowVersionTrigger,
DeleteCoreWorkflowVersionStep, DuplicateCoreWorkflow, RunCoreWorkflowVersion, GetCoreWorkflowRunInsights.

### 2.10 AI chat — `modules/ai-chat/` 605/27 714 + `modules/inbox/` 140 + `modules/legacy-ai-chat/`
Sub-packages: composer, messages, message-content, thinking-steps, tool-calls, suggested-prompts, threads,
dictation, usage, workspace-setup. Ops: SendChatMessage, GetChatMessages, FindChatMessageAgent,
RetryChatMessage, StopAgentChatStream, AnswerToolCall, AssignAgentChatThread, SnoozeAgentChatThreadInChannel,
MoveAgentChatThreadToChannel, MarkAgentChatThreadAsDoneInChannel, CreateAgentChatChannel,
GetAgentChatChannels, GetAgentChatChannelThreadIds, GetAgentChatOpenThreadsSummary,
AddAgentChatMessageReaction, FindManyAgents, FindManySkills, GetToolIndex, GetToolInputSchema,
GetAiChatUsage, GetAgentRunInsights.

### 2.11 Dashboards / page layouts — `modules/page-layout/` 811/41 172
`PageLayoutType` = RECORD_INDEX, RECORD_PAGE, DASHBOARD, STANDALONE_PAGE, RECORD_FORM;
`WidgetType` = VIEW, IFRAME, FIELD, FIELDS, GRAPH, STANDALONE_RICH_TEXT, TIMELINE, TASKS, NOTES, FILES,
EMAILS, CALENDAR, FIELD_RICH_TEXT, WORKFLOW, WORKFLOW_VERSION, WORKFLOW_RUN, FRONT_COMPONENT,
RECORD_TABLE, EMAIL_THREAD, CALL_RECORDING_SUMMARY, CALL_RECORDING_TRANSCRIPT, MESSAGE_CAMPAIGN_BODY,
MESSAGE_CAMPAIGN_DETAILS, FORM_FIELD, CHAT_THREADS; tab layout modes GRID, VERTICAL_LIST, CANVAS.
Widgets `modules/page-layout/widgets/*`: calendar, call-recording(+summary/transcript), chat, chat-threads,
email-thread, field(s), files, front-component, graph (bar/line/pie via `@nivo`), iframe, message-campaign,
messages, notes, record-table, standalone-rich-text, tasks, timeline, workflow.
Ops: FindOnePageLayout, FindOnePageLayoutType, FindAllRecordPageLayouts, FindAllRecordFormPageLayouts,
FindAllStandalonePageLayouts, UpdatePageLayoutWidget, UpdatePageLayoutWithTabsAndWidgets,
UpsertFieldsWidget, UpsertViewWidget, ResetPageLayout{,Tab,Widget}ToDefault, BarChartData, LineChartData,
PieChartData.

### 2.12 Activities: timeline, notes, tasks, mail, calendar — `modules/activities/` 362/19 512
`timeline-activities/` (rows, types, filters), `notes/`, `tasks/`, `emails/` (recipients, editor, blocks),
`calendar/`, `files/`; dynamic activity queries in `activities/graphql/operation-signatures/`.
Ops: FindManyTimelineActivityTypes, UpdateTimelineActivityTypeIsActive, ResetTimelineActivityType,
SearchTimelineThreads, SearchTimelineCalendarEvents, GetTimelineThreadsFromObjectRecord,
GetTimelineCalendarEventsFromObjectRecord, GetTimelineThreadChannelsFromObjectRecord,
GetMessageThreadChannel, SendEmail, SendAppChannelMessage, MarkAppChannelThreadAsRead,
SendMessageCampaign(+Test), CancelMessageCampaign, PreviewMessageCampaignSend, CreateCalendarEvent,
UnsubscribeTopics.
Notes/tasks are records of `StandardObjectNameSingular.Note` / `.Task`
(`packages/twenty-shared/src/types/StandardObjectNameSingular.ts`: person, company, opportunity, task,
note, message, messageThread, messageCampaign, messageList, calendarEvent, attachment, callRecording,
timelineActivity, workflow, workflowVersion, workflowRun, agentChatThread, workspaceMember, dashboard) —
created through the generic record mutations, not dedicated ops.
Mail UI `activities/emails/` + `modules/inbox/`; calendar UI `activities/calendar/` +
`page-layout/widgets/calendar/`.

### 2.13 Misc worth porting
Cmd-K shell, information banners (maintenance/reconnect), keyboard-shortcut menu, layout-customization
(sidebar sections), spreadsheet import (CSV/XLSX), record export (CSV), log console, media-access,
mention (@-mentions), geo-map, captcha (Turnstile on auth), related-record-paths, validation-rules,
logic-functions (Monaco), front-components, marketplace, mcp-servers, localization (Lingui),
sign-in-background-mock.

---

## 3. Design system

Two layers: the published package `packages/twenty-ui` (primitives + tokens) and the app-local layer
`packages/twenty-front/src/modules/ui` (618 files: field, input, layout, navigation, suggestion,
utilities, feedback, theme, icon).

### 3.1 `twenty-ui` package
- `src/primitives/` (618 files) on `@base-ui/react` v1.9 + SCSS modules: input (Button, ButtonGroup,
  Input, InputGroup, NumberField, Textarea, Checkbox, Radio, RadioGroup, Switch, Select, Slider,
  Autocomplete, Field); data-display (Avatar, Badge, Chip, ColorSample, Status, Tag); feedback (Banner,
  Loader, Progress, ProgressBar, ProgressRing, Skeleton); surfaces (AlertDialog, Card, Dialog, Popover,
  Tooltip); navigation (Breadcrumb, ListItem, Tabs); typography (Heading, Text, Shortcut,
  OverflowingTextWithTooltip); layout (Collapsible, DirectionProvider, ResizeHandle, Separator);
  accessibility (focus management).
- `src/components/` (503 files): input (MainButton, LightButton, IconButton, LightIconButton, SearchInput,
  DatePicker, CountrySelect, CurrencyPicker, EmojiPicker, ImageInput, NumberStepper, PhoneCountryPicker,
  ReactionButton, SegmentedControl, FormFieldRow); navigation (Dropdown menu/picker/panel, MenuItem,
  ListItemButton, TabButton, CountPager); data-display (AssistantAvatar, AvatarGroup, CommandBlock,
  DetailsRow, JsonTree, MetricRow, StackIcon, TintedIconTile); feedback (Callout, InlineBanner, Toast,
  Toaster); layout (Section, LabeledSeparator, OverflowingList, AnimatedIconCrossfade); settings
  (SettingsRow); code-editor (CodeEditor + Header, Monaco).
- `src/theme/` (ThemeProvider, theme-light.css / theme-dark.css, token tree); `src/icon/`; `design-tokens/`
  is the token source of truth (TS), compiled to CSS + TS by `scripts/generateThemeTokens.ts`
  (`nx generateTokens twenty-ui`).
- Key APIs: `Button` variant `solid|outline|ghost|soft|link` × color `neutral|accent|danger|success|
  inherit` × size `sm|md`, plus fullWidth, loading(+loadingPosition start|center|end), elevated,
  startIcon/endIcon, shortcut; `solid`+`inherit` throws; md = 32px tall, sm = 24px.
  `Chip` variant `solid|soft|ghost|plain|outline`, size sm|md, color primary|secondary, shape square|round,
  weight regular|medium, startElement/endElement, maxWidth. `Tag` color = any of 25 theme colors +
  transparent, state variant `solid|soft|outline|ghost` + weight + truncate. `Avatar` shape round|square,
  variant soft|outline, deterministic `colorSeed`. `Status` = canonical record/process status.

### 3.2 Theme tokens (`design-tokens/*` → `theme-light.css` / `theme-dark.css`)
CSS custom properties `--t-*` scoped by `.light` / `.dark` on `<html>` (ThemeProvider toggles both).
twenty-front reads them via `themeCssVariables` (`packages/twenty-ui/src/theme/themeCssVariables.ts`)
inside Linaria `styled`; twenty-ui SCSS modules read `var(--t-...)` directly.
- **Spacing** (4px grid): `--t-spacing-0..32` = 0,4,…,128px; `0_5`=2px, `1_5`=6px; `25`=100px,
  `26..31`=104..124px; `--t-spacing-multiplicator: 4`.
- **Typography**: Inter everywhere (`--t-font-family: Inter, sans-serif`), **root font-size 13px**
  (`packages/twenty-front/src/index.css`), DM Mono for code (`--t-code-font-family`). Sizes xxs .625rem,
  xs .85, sm .92, md 1rem(=13px), lg 1.23, xl 1.54, xxl 1.85; weights 400/500/600;
  `--t-font-color-{primary,secondary,tertiary,light,extra-light,inverted,on-accent,danger}`.
- **Radii**: xs 2px, sm 4px, md 8px, lg 16px, xl 20px, xxl 40px, pill 999px, rounded 100%;
  round variants sm-round 4, md-round 8, lg-round 12 (avatars/chips/switch handles never double).
  Under `@supports (corner-shape: squircle)` radii **double** (xs 4, sm 8, md 16, lg 32, xl 40, xxl 80)
  and `corner-shape: var(--t-corner-shape, superellipse(2.2))` is applied globally; nested radii must be
  concentric: `calc(var(--t-border-radius-md) - var(--t-spacing-1))`.
- **Shadows** (`design-tokens/boxShadow.ts`): color, light (0 2px 4px + 0 0 4px), sidebar, bottomPanel
  (0 0 20px), strong (2px 4px 16px), underline (0 1px 0), superHeavy (0 0 8px + 0 8px 64px −16px +
  0 24px 56px −16px). Shadows only on floating layers (dropdowns, popovers, dialogs).
- **Colors**: backgrounds `--t-background-{primary,secondary,tertiary,quaternary}`,
  `inverted-primary/secondary`, `danger`, `danger-solid`, `transparent-{primary,secondary,strong,medium,
  light,lighter,danger,blue,orange,success}`, `overlay-{primary,secondary,tertiary}`, `radial-gradient(+hover)`,
  `noisy` (data-uri). Borders `--t-border-color-{strong,medium,light,secondary-inverted,inverted,danger,
  blue,transparent-strong}`. Accent ramp `--t-accent-accent1..12` (+ `accent-primary/secondary/tertiary/
  quaternary`, `accent3570`, `accent4060`); light accent9 = product blue `--t-color-blue`
  `color(display-p3 0.276 0.384 0.837)`. Palette: 25 hues × 12 steps `--t-color-{hue}{1..12}` + base
  `--t-color-{hue}` (`src/theme/constants/MainColorNames.ts`: red, ruby, crimson, tomato, orange, amber,
  yellow, lime, grass, green, jade, mint, turquoise, cyan, sky, blue, iris, violet, purple, plum, pink,
  bronze, gold, brown, gray) + neutrals (mauve, slate, sage, olive, sand, gray); light ramp light→dark,
  dark inverted. Grays `--t-gray-scale-gray1..12`. Tag colors `--t-tag-text-{hue}` /
  `--t-tag-background-{hue}` (all 25). Snack bar `--t-snack-bar-{success,error,warning,info,default}-
  {color,background-color}`. Code `--t-code-text-{gray,sky,pink,orange,green}`. Illustration icons
  `--t-illustration-icon-{color,fill}-{blue,gray}`. Icons: `--t-icon-size-{sm,md,lg,xl}` = 14/16/20/24,
  `--t-icon-stroke-{sm,md,lg}` = 1.6/2/2.5. Animation `--t-animation-duration-{instant,fast,normal,slow}` =
  .075/.15/.3/1.5s. Modal `--t-modal-size-{sm,compact,md,lg,xl,fullscreen}`. Table metrics
  `--t-table-horizontal-cell-{margin,padding}` = 8px, `--t-table-checkbox-column-width` = 32px.
- **Light**: background primary 1 / secondary .988 / tertiary .945 / quaternary .922; font primary .2 /
  secondary .4 / tertiary .6 / light .702 / extra-light .8; border strong .839 / medium .922 / light .945.
  **Dark**: background primary .09 / secondary .106 / tertiary .114 / quaternary .133; font primary .922 /
  secondary .702 / tertiary .506 / light .4 / extra-light .298; border strong .282 / medium .133 / light .114.
- **Layout metrics** (twenty-front): `APP_HEADER_HEIGHT=48`, `PAGE_BAR_MIN_HEIGHT=32`, drawer
  {180,220,350} + collapsed 48, side panel {320,400,600}, table row 32, chat max width 768, menu max
  height 176px, `MOBILE_VIEWPORT=768`.

### 3.3 Icons
- Base set **Tabler** `@tabler/icons-react` ^3.31, re-exported by
  `packages/twenty-ui/src/icon/components/TablerIcons.ts` (471 export lines) and published as
  `twenty-ui/icon` (never import the package directly).
- Custom product icons (`src/icon/components/`): IconAddressBook, IconLock, IconGmail, IconGoogle,
  IconMicrosoft, IconGoogleCalendar, IconMicrosoftCalendar, IconMicrosoftOutlook, IconEuStars,
  IconSparkle2, IconTrashXOff, IconTwentyStar(+Filled), IconChartBarHorizontal, IconRelationManyToOne,
  brand icons (Anthropic, Gemini, Groq, Hubspot, Mistral, Pipedrive, Salesforce, TypesafeAi, Xai,
  Openai, ModelClaude), ThinkingOrbitLoaderIcon.
- **Illustration icons** (27, one per field type): IllustrationIcon{Array,CalendarEvent,CalendarTime,
  Currency,File,Json,Link,Mail,ManyToMany,Map,Numbers,OneToMany,OneToOne,Phone,Setting,Star,Tag,Tags,
  Text,Toggle,Uid,User,Wrapper}.
- **Icon dictionary** `src/icon/constants/TwentyIconDictionary.ts` (674 lines; prose map in
  `icon-dictionary.md`): Data model `IconHierarchy`, Object `IconBox`, Field `IconListSearch`,
  Record `IconAddressBook`, Person `IconUser`, Company `IconBuildingSkyscraper`, Opportunity
  `IconTargetArrow`, Task `IconCheckbox`, Note `IconNotes`, View `IconTable`, Kanban `IconLayoutKanban`,
  Group by `IconLayoutList`, Filter `IconFilter`, Sort `IconArrowsSort`, Layout `IconAppWindow`,
  Layout page `IconPerspective`, Widget `IconLayoutGridAdd`, Side panel `IconLayoutSidebarRight`.
- Sizes 16px default (14px in dense rows/cells, max 20px for a lone icon); stroke 1.6.

---

## 4. State architecture

### 4.1 Jotai (there is **no Recoil** in this codebase)
All client state is Jotai atoms created by `modules/ui/utilities/state/jotai/` helpers:
`createAtomState` (global; optional localStorage/sessionStorage/cookie persistence —
`createJotaiCookieStorage`, `createIndexedDbBackedJotaiStorage`, `validateInitFn`, `scope:'routed-flow'`
released by `routedFlowStateScopeRegistry`), `createAtomFamilyState` (keyed by id),
`createAtomSelector` / `createAtomWritableSelector`, `createAtomComponentState` /
`createAtomComponentFamilyState` (per component-instance id via context), and the matching component
selectors. Hooks (19 files): `useAtomStateValue`, `useSetAtomState`, `useAtomFamilyStateValue`,
`useSetAtomFamilyState`, `useAtomComponentStateValue`, `useSetAtomComponentState`,
`useAtomComponentSelectorValue`, … Global store `jotai/jotaiStore.ts` (`useStore()` for imperative
reads/writes); component-instance registry `component-state/utils/globalComponentInstanceContextMap.ts`.

### 4.2 Metadata store (drives the whole UI)
`modules/metadata-store/`: `states/metadataStoreState.ts` = a **family atom per entity kind**, each
entry `{current[], draft[], status:'empty'|'draft-pending'|'up-to-date', currentCollectionHash?,
draftCollectionHash?}`. 27 kinds (`ALL_METADATA_ENTITY_KEYS`): objectMetadataItems, fieldMetadataItems,
indexMetadataItems, views, viewFields, viewFilters, viewSorts, viewGroups, viewFilterGroups,
viewFieldGroups, pageLayouts, pageLayoutTabs, pageLayoutWidgets, logicFunctions, navigationMenuItems,
commandMenuItems, frontComponents, webhooks, roles, roleTargets, agents, skills,
rowLevelPermissionPredicates(+Groups), applications.
`graphql/queries/findMinimalMetadata.ts` → `FIND_MINIMAL_METADATA` (objectMetadataItems, views,
collectionHashes). Hooks: `useLoadMinimalMetadata`, `useLoadStaleMetadataEntities`,
`useInvalidateMetadataStore`, `useResyncMetadataStore`, `useUpdateMetadataStoreDraft`.
Gate: `modules/metadata-store/components/MinimalMetadataGate.tsx` blocks the workspace router until loaded.

### 4.3 Object/field metadata selectors — `modules/object-metadata/states/`
`objectMetadataItemsSelector`, `objectMetadataItemsByIdMapSelector`, `objectMetadataItemsByNamePluralMapSelector`,
`flatObjectMetadataItemsSelector`, `flattenedFieldMetadataItemsSelector`, `fieldMetadataItemByIdSelector`,
`fieldMetadataItemByIdMapSelector`, `labelIdentifierFieldMetadataItemSelector`,
`availableFieldMetadataItemsForFilterFamilySelector`, `availableFieldMetadataItemsForSortFamilySelector`,
`isFieldMetadataItemFilterableAndSortableSelector`, `objectPermissionsByObjectMetadataIdSelector`.
Queries: `FIND_MANY_OBJECT_METADATA_ITEMS` (`query ObjectMetadataItems`, first 1000) with
`OBJECT_METADATA_FRAGMENT` (object ids/names/labels/color/icon/isActive/isSystem/isRemote/
`labelIdentifierFieldMetadataId`/`imageIdentifierFieldMetadataId`/`openRecordIn`/`duplicateCriteria`/
`searchFieldMetadataList`/`indexMetadataList`/`fieldsList`; field id, type, name, label, description, icon,
isActive/isSystem/isNullable/isUnique/isSearchable, defaultValue, options, settings, `relation{type,
source/targetObjectMetadata, source/targetFieldMetadata}`, `morphRelations`); plus `OBJECT_RECORD_COUNTS`,
`MOSTLY_EMPTY_FIELD_METADATA_IDS`.

### 4.4 How metadata drives the record UI
1. Object metadata → fields; each field becomes a `FieldDefinition`
   (`modules/object-record/record-field/ui/types/FieldDefinition.ts`): `{fieldMetadataId, label,
   showLabel?, labelWidth?, iconName, type: FieldMetadataType, metadata: T, defaultValue?,
   editButtonIcon?, isUIEditable?}` via `object-metadata/utils/formatFieldMetadataItemAsFieldDefinition.ts`;
   `formatFieldMetadataItemAsColumnDefinition` adds `{position (0 = label identifier), size: 100,
   isLabelIdentifier, isVisible: true}` → table column.
2. `ViewField` (visible columns, order, size, aggregate) + object metadata → column set;
   `isFieldCellSupported` decides which field types can be a cell.
3. Cells and show-page fields dispatch on the field type via guards in
   `record-field/ui/types/guards/` (`isFieldText`, `isFieldNumber`, …).
Field types (`packages/twenty-shared/src/types/FieldMetadataType.ts`): ACTOR, ADDRESS, ARRAY, BOOLEAN,
CURRENCY, DATE, DATE_TIME, EMAILS, FILES, FULL_NAME, LINKS, MORPH_RELATION, MULTI_SELECT, NUMBER,
NUMERIC, PHONES, POSITION, RATING, RAW_JSON, RELATION, RICH_TEXT, SELECT, TEXT, TS_VECTOR, UUID.
- Display (`record-field/ui/meta-types/display/components/`): Text, Uuid, Number, Date, DateTime, Select,
  MultiSelect (+MultiSelectFieldDisplayTag), RelationToOne, RelationFromMany, MorphRelationManyToOne,
  MorphRelationOneToMany, Currency, FullName, Address, Links, Emails, Phones, Boolean, Rating, Json
  (RAW_JSON), RichText, Actor, Array, Files, Chip (label identifier), CoreRecordReference, Forbidden.
- Input (`record-field/ui/meta-types/input/components/`): Text, Number, Date, DateTime, Select, MultiSelect,
  Boolean, Rating, Currency, FullName, Address, Links (+DomainFavicon/EditInputs/EditPreview), Emails,
  Phones, Files (+DropZone/LabelInput), Array, RawJson, RichText (+Editor), RelationManyToOne,
  RelationOneToMany, MorphRelationManyToOne, MorphRelationOneToMany, plus shared multi-item list
  (MultiItemField{InputRow,List,MenuItem,TextInput,AddButton,SaveButton,SearchHeader,EditedItem,
  ItemSeparator}).
- Composite sub-fields (`packages/twenty-shared/src/constants/CompositeFieldTypeSubFieldsNames.ts`):
  CURRENCY{amountMicros,currencyCode}; EMAILS{primaryEmail,additionalEmails};
  LINKS{primaryLinkUrl,primaryLinkLabel,secondaryLinks};
  PHONES{primaryPhoneCallingCode,primaryPhoneCountryCode,primaryPhoneNumber,additionalPhones};
  FULL_NAME{firstName,lastName}; ADDRESS{addressStreet1,addressStreet2,addressCity,addressState,
  addressCountry,addressPostcode,addressLat,addressLng}; ACTOR{source,name,workspaceMemberId,context};
  RICH_TEXT{blocknote,markdown,tiptap}.
- Validation: one Zod schema per field type in `record-field/ui/validation-schemas/` (address, array,
  currencyCode, currency default, emails, files, fullName, links, multiSelect value+default, number
  value+default, phones, select, text, …). Field metadata shapes in `record-field/ui/types/FieldMetadata.ts`
  (e.g. `FieldTextMetadata.placeHolder`, `FieldNumberMetadata.settings.decimals|type` (number|percentage|
  shortNumber), `FieldDateMetadata.settings.displayFormat` RELATIVE|USER_SETTINGS|CUSTOM +
  `customUnicodeDateFormat`).

### 4.5 Other state layers
- `modules/context-store/` — current context shared by drawer and page: currentObjectMetadataItemId,
  currentViewId, currentViewType, currentPageType, currentCoreObjectName, filters, filterGroups,
  anyFieldFilterValue, numberOfSelectedRecords, recordIdsInSelectionOrder, recordShowParentView,
  targetedRecordsRule.
- Feature-local atoms: `object-record/record-index/states/`, `record-group/states/`,
  `record-selection/states/`, `views/states/`, `side-panel/states/`, `command-menu-item/states/`.
- `modules/client-config/states/` — ~45 atoms fed by the `/client-config` REST payload (apiConfig,
  feature flags, captcha, auth providers, billing, AI models/tiers, appVersion).
- `modules/auth/states/` — currentUser, currentWorkspace, currentWorkspaceMember, availableWorkspaces,
  loginToken, signInUpStep, isImpersonating, billingCheckoutSession, …
- `modules/apollo/` — Apollo Client v4: upload link (`apollo-upload-client`, `credentials:'include'`),
  REST link to `${SERVER_BASE_URL}/rest`, logger link, streaming REST link, SSE link for
  `subscription OnEventSubscription`; optimistic effects in `apollo/optimistic-effect/` (incl. group-by).
- `modules/sse-db-event/` — SSE client: `AddQueryToEventStream`/`RemoveQueryFromEventStream` bind a live
  query to a stream; events patch the Apollo cache and dispatch browser events
  (`useDispatchObjectRecordEventsFromSseToBrowserEvents`).
- `modules/localization/` — Lingui (`@lingui/react`) catalogs.

### 4.6 Data flow (record index)
ObjectMetadataItems + views(+viewFields) → `RecordIndexContainer` → view bar (`modules/views/components/`
`ViewBar`) → filters/sorts/groups from URL query params (`QueryParamsFiltersEffect`,
`QueryParamsSortsEffect`) → generated `FindMany<Objects>` → cache → virtualized `RecordTable` /
`RecordBoard` / `RecordCalendar` → cells render `FieldDisplay` → click → `FieldInput` →
`updateOneRecord` mutation + optimistic cache write + SSE confirmation.

---

## 5. Email templates (`packages/twenty-emails`)

Server-side React Email package (React 19 + `react-email` ^6.11 + `@react-email/ui` + Lingui);
`src/index.ts` re-exports `render`, `toPlainText` from a single pinned react-email version.
Shared parts `src/components/`: `BaseEmail` (Html + Container(width 290) + `BaseHead` + `Logo` +
children + `Footer`), `Title`, `SubTitle`, `MainText`, `HighlightedText`, `HighlightedContainer`,
`CallToAction`, `Link`, `ShadowText`, `WhatIsTwenty`. `src/common-style.ts` `canvasTheme`:
font `Trebuchet MS` (web-safe; Inter unavailable in mail), sizes 12/13/16/24px, line-height 20px,
weights 400/600, radius 4/8px, grays `#000000`→`#ffffff`, accent `#5e90f2`.
Templates `src/emails/*.email.tsx` (17) — props beside `locale`:
send-invite-link (`link`, `workspace{name,logo}`, `sender{email,firstName,lastName}`, `serverUrl`,
CTA "Accept invite") · send-email-verification-link (`link`, `isEmailUpdate?`) ·
password-reset-link (`duration`, `hasPassword`, `link`) · password-update-notify (`userName`, `email`,
`link`) · billing-trial-ending (`userName`, `workspaceDisplayName`, `trialEndsAt`, `dataRetentionDays`,
`link`) · billing-trial-converting (`…`, `interval:'month'|'year'`, `link`) ·
billing-subscription-renewing (`userName`, `workspaceDisplayName`, `renewsAt`, `link`) ·
clean-suspended-workspace (`daysSinceInactive`, `userName`, `workspaceDisplayName`) ·
warn-suspended-workspace (`daysSinceInactive`, `inactiveDaysBeforeDelete`, `userName`,
`workspaceDisplayName`, `link`) · server-admin-access-changed (`actorName`, `targetName`,
`targetEmail`, `canAccessFullAdminPanel`, `canImpersonate`) · two-factor-authentication-backup-code-used
(`workspaceDisplayName`, `remainingBackupCodeCount`) · …-backup-codes-regenerated (`workspaceDisplayName`) ·
…-recovery-code-issued (`actorName`, `workspaceDisplayName`, `expiresAt`) · …-removed (`workspaceDisplayName`) ·
…-reset (`workspaceDisplayName`) · validate-approved-access-domain (`link`, `domain`, `workspace`,
`sender`, `serverUrl`) · test.
Rich-email renderer `src/utils/email-renderer/`: TipTap → React Email; marks (bold, italic, link, strike,
underline), nodes (blockquote, bullet-list, button, columns, divider, hard-break, heading, html, image,
list-item, ordered-list, paragraph, section, text, variable-tag), renderers (`render-mark.tsx`,
`render-node.tsx`), utils (block-spacing, block-style, inherited-typography, safe-url).
`src/utils/render-email.ts` throws `EmailRenderError` on empty body or React's errored-Suspense marker
`<!--$!-->`. i18n: `src/locales/*.po` — 35 locales (af-ZA, ar-SA, ca-ES, cs-CZ, da-DK, de-DE, el-GR,
en, es-ES, fa-IR, fi-FI, fr-FR, he-IL, hu-HU, hy-AM, it-IT, ja-JP, ko-KR, nl-NL, no-NO, pl-PL,
pseudo-en, pt-BR, pt-PT, ro-RO, ru-RU, sr-Cyrl, sr-Latn, sv-SE, tr-TR, uk-UA, vi-VN, zh-CN, zh-TW).
Porting note: rendering is a **server** concern (Rust renders HTML); the Leptos UI only calls
`ResendEmailVerificationToken` / `EmailPasswordResetLink`.

---

## 6. Recommended MVP-first build order (Leptos)

Server already exposes the Twenty-compatible GraphQL schema (ADR-004), so the UI work is
theme+primitives → auth → metadata → record index → record page. Each stage ships a runnable app.
Sizes = estimated Rust LOC (excl. generated GraphQL types) and effort.

**Stage 0 — Foundations (design system + shell) ~7 000 LOC, 1.5 wk.**
Tokens (light/dark CSS var sets copied verbatim from `theme-{light,dark}.css`), theme switcher.
Primitives: Button (5 variants × 4 colors × 2 sizes), Input, Textarea, Select, Checkbox, Switch, Radio,
Field, Tabs, Tooltip, Popover, Menu/Dropdown, Dialog, AlertDialog, Card, Banner, Callout, Toast/Toaster,
Loader, Skeleton, Progress, Avatar, Badge, Chip, Tag, Status, Heading/Text/Shortcut, Breadcrumb, ListItem.
Icons: Tabler subset (~120 SVG) + 27 illustration icons + icon-dictionary mapping.
Shell: router (`leptos_router`), layout (drawer 220 / header 48 / page pane), main-vs-side-panel
surface, error boundary, toaster. Client: HTTP client with cookies, GraphQL client, `/client-config` store.

**Stage 1 — Auth + workspace bootstrap ~3 500 LOC, 1 wk.**
Pages `/welcome` (email→password→2FA→workspace select/create), `/verify`, `/verify-email`,
`/reset-password/:token`, `/invite/:hash`, `/authorize`, 404. Mutations: CheckUserExists,
GetLoginTokenFromCredentials, getAuthTokensFromLoginToken, SignIn, SignUp, SignUpInNewWorkspace,
SignUpInWorkspace, SignOut, ResendEmailVerificationToken, EmailPasswordResetLink,
UpdatePasswordViaResetToken, VerifyEmailAndGetLoginToken, getAuthTokensFromOTP, GetAuthorizationUrlForSSO.
State: current user/workspace, sign-in-up step machine, captcha widget.

**Stage 2 — Metadata client + record index (table only) ~9 000 LOC, 2 wk.**
Metadata store (current/draft + collection hashes) + selectors; dynamic `FindMany<Objects>` query
builder (metadata → GraphQL string; also needed server-side, ADR-004). Table: virtualized 32px rows,
sticky header, column resize + visibility menu, cell display + inline edit (TEXT, NUMBER, DATE,
DATE_TIME, SELECT, MULTI_SELECT, BOOLEAN, RELATION, CURRENCY, EMAILS, PHONES, LINKS, UUID, RATING),
checkbox column, selection, empty/loading states, label-identifier chip, cursor pagination + infinite
scroll + total count.

**Stage 3 — Record page ~4 500 LOC, 1 wk.** Route `/object/:singular/:id`; header (icon, title,
favorite, actions), field sections, inline editing, created-by/at bar, breadcrumb, prev/next,
read-only states, field description tooltips, relation/file pickers.

**Stage 4 — Views, filters, sort, group-by, nav drawer ~6 500 LOC, 1.5 wk.**
View bar (view picker, filter/sort/group menus, hidden-fields menu, page title); per-type filter
builders with the operand matrix + any-field search + advanced AND/OR groups; sort menu; group tabs
with counts and aggregates (COUNT/SUM/AVG/MIN/MAX/…); nav drawer (objects, views, favorites, folders,
links, multi-workspace switcher, collapsible sections, mobile bar); view CRUD ops.

**Stage 5 — Command menu + side panel ~5 000 LOC, 1.5 wk.**
Cmd-K: search, sections (pinned → selection → current object → ask & find → create record → workspace →
go to → Ask AI fallback), keyboard nav, hotkeys, pinned items in page headers; engine commands (create,
export, delete/restore/destroy, merge, bulk update, add/remove favorite, share, go to X).
Side panel: open/close/expand, top bar, back, routed pages (record preview, search, update records,
create related record), width persistence.

**Stage 6 — Kanban, calendar, list, drag & drop ~4 500 LOC, 1 wk.** Kanban columns from groups + cards +
drag between columns (patch group value) + column width; calendar day/week/month + drag to another day;
list view; record DnD in the table; bulk update from selection.

**Stage 7 — Activities (timeline, notes, tasks) + rich text ~6 000 LOC, 1.5 wk.** Timeline on the record
page (notes, tasks, messages, calendar events, files) with type filters, composer, attachments; notes/tasks
as records; TipTap-equivalent editor (links, mentions, lists, markdown import/export).

**Stage 8 — Settings (data model, members, general, accounts) ~8 000 LOC, 2 wk.** Settings shell (section
stack, tabs, breadcrumb, table lists); objects/fields/indexes/validation-rules/translations; members +
roles matrix; general (name/logo/subdomain); profile; theme/experience.

**Stage 9 — Page layouts & dashboards ~7 000 LOC, 1.5 wk.** Tabs + widgets, grid/vertical-list/canvas,
widget chrome, record-page layout (field sections), standalone pages; widgets: fields, record table,
notes, tasks, files, timeline, rich text, charts (bar/line/pie), iframe, workflow.

**Stage 10 — AI chat + inbox + agents ~6 500 LOC, 1.5 wk.** Thread view, composer (text, mentions,
files, dictation), streaming (SendChatMessage + SSE), thinking steps, tool-call chips, reactions, retry,
new thread; side-panel chat; inbox (channels, thread list, unread); agent list/detail; skills.

**Stage 11 — Workflows ~7 000 LOC, 1.5 wk.** Workflow list; diagram editor (nodes/edges, drag to connect,
zoom/pan); step inspector (form/code/HTTP/if-else/delay/AI agent); version history; run view; test-run.

**Stage 12 — Onboarding, mail/calendar, polish ~5 000 LOC, 1 wk.** Onboarding steps, getting started;
mail (message list, thread, composer); calendar (channels, events); CSV/XLSX import + CSV export;
information banners; keyboard-shortcut menu.

**Stage 13 — Long tail (optional) ~6 000 LOC, 1 wk.** Applications & marketplace, front components,
logic functions (Monaco), MCP servers, API keys & webhooks, billing/usage, admin panel, domains,
SSO/security, DPA/legal, log console, geo-map; port of `packages/twenty-companion` (Electron desktop
app: calendar events, call recording, tray menu) as a Tauri app.

MVP through Stage 6 ≈ **40 000 Rust LOC** covering auth, metadata, record index (table/kanban/calendar/
list), record page, views/filters/sort/group-by, nav drawer with favorites, command menu and side panel —
i.e. Twenty's whole daily-driver surface.

---

## 7. Quick file-path index

- Routers: `modules/app/hooks/useCreate{Root,Workspace}AppRouter.tsx`,
  `modules/app/routing/utils/createWorkspaceRouteObjects.tsx`, `modules/app/components/SettingsRoutes.tsx`.
- Path enums: `packages/twenty-shared/src/types/{AppPath,SettingsPath,SidePanelPages}.ts`; pages `pages/**`.
- Records: `modules/object-record/record-index/components/RecordIndexContainer.tsx`,
  `record-table/**`, `record-board/**`, `record-calendar/**`, `record-list/**`, `record-show/**`,
  `record-field/ui/**`, `record-group/**`, `record-filter/**`.
- Metadata & queries: `modules/metadata-store/**`, `modules/object-metadata/**`,
  `modules/object-record/utils/generateFindManyRecordsQuery.ts`,
  `modules/object-metadata/utils/mapObjectMetadataToGraphQLQuery.ts`.
- Field model: `packages/twenty-shared/src/types/FieldMetadataType.ts`,
  `packages/twenty-shared/src/constants/CompositeFieldTypeSubFieldsNames.ts`,
  `packages/twenty-shared/src/utils/filter/utils/filterOperandsMap.ts`.
- State: `modules/ui/utilities/state/jotai/**`, `modules/context-store/**`, `modules/apollo/**`,
  `modules/sse-db-event/**`.
- Command menu / side panel / nav: `modules/command-menu/**`, `modules/command-menu-item/**`,
  `modules/side-panel/**`, `modules/navigation/**`, `modules/navigation-menu-item/**`.
- Engines & features: `modules/workflow/**`, `modules/object-core/**`, `modules/ai-chat/**`,
  `modules/inbox/**`, `modules/page-layout/**`, `modules/activities/**`, `modules/onboarding/**`,
  `modules/getting-started/**`, `modules/auth/**`, `modules/settings/**`.
- Design system: `packages/twenty-ui/design-tokens/**`, `packages/twenty-ui/src/theme/theme-{light,dark}.css`,
  `packages/twenty-ui/src/primitives/**`, `packages/twenty-ui/src/components/**`,
  `packages/twenty-ui/src/icon/**` (`icon-dictionary.md`, `constants/TwentyIconDictionary.ts`), `DESIGN.md`.
- Emails & desktop: `packages/twenty-emails/src/**`, `packages/twenty-companion/src/**`.
