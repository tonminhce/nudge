--
-- PostgreSQL database dump
--

\restrict 6W0R2gj9jRAjN853g1GIhxmUoG2xs8igFBTxhcqraV00JKi70yKCCjefkFn6yQR

-- Dumped from database version 16.16 (Debian 16.16-1.pgdg13+1)
-- Dumped by pg_dump version 16.16 (Debian 16.16-1.pgdg13+1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: core; Type: SCHEMA; Schema: -; Owner: -
--

CREATE SCHEMA core;


--
-- Name: applicationVariable_scope_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."applicationVariable_scope_enum" AS ENUM (
    'WORKSPACE',
    'USER'
);


--
-- Name: calendarChannel_contactautocreationpolicy_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."calendarChannel_contactautocreationpolicy_enum" AS ENUM (
    'AS_PARTICIPANT_AND_ORGANIZER',
    'AS_PARTICIPANT',
    'AS_ORGANIZER',
    'NONE'
);


--
-- Name: calendarChannel_syncstage_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."calendarChannel_syncstage_enum" AS ENUM (
    'PENDING_CONFIGURATION',
    'CALENDAR_EVENT_LIST_FETCH_PENDING',
    'CALENDAR_EVENT_LIST_FETCH_SCHEDULED',
    'CALENDAR_EVENT_LIST_FETCH_ONGOING',
    'CALENDAR_EVENTS_IMPORT_PENDING',
    'CALENDAR_EVENTS_IMPORT_SCHEDULED',
    'CALENDAR_EVENTS_IMPORT_ONGOING',
    'FAILED'
);


--
-- Name: calendarChannel_syncstatus_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."calendarChannel_syncstatus_enum" AS ENUM (
    'NOT_SYNCED',
    'ONGOING',
    'ACTIVE',
    'FAILED_INSUFFICIENT_PERMISSIONS',
    'FAILED_UNKNOWN'
);


--
-- Name: calendarChannel_visibility_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."calendarChannel_visibility_enum" AS ENUM (
    'METADATA',
    'SHARE_EVERYTHING'
);


--
-- Name: calendarChannel_webhooksubscriptionstatus_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."calendarChannel_webhooksubscriptionstatus_enum" AS ENUM (
    'PENDING',
    'ACTIVE',
    'FAILED',
    'EXPIRED'
);


--
-- Name: commandMenuItem_availabilitytype_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."commandMenuItem_availabilitytype_enum" AS ENUM (
    'GLOBAL',
    'GLOBAL_OBJECT_CONTEXT',
    'RECORD_SELECTION',
    'FALLBACK'
);


--
-- Name: connectionProvider_scope_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."connectionProvider_scope_enum" AS ENUM (
    'WORKSPACE',
    'USER'
);


--
-- Name: dataSource_type_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."dataSource_type_enum" AS ENUM (
    'postgres'
);


--
-- Name: dpaAgreement_type_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."dpaAgreement_type_enum" AS ENUM (
    'CLICK_THROUGH',
    'SIGNED'
);


--
-- Name: emailingDomain_status_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."emailingDomain_status_enum" AS ENUM (
    'PENDING',
    'VERIFIED',
    'FAILED',
    'TEMPORARY_FAILURE'
);


--
-- Name: emailingDomain_tenantstatus_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."emailingDomain_tenantstatus_enum" AS ENUM (
    'ACTIVE',
    'PAUSED',
    'SANDBOX'
);


--
-- Name: emailingDomain_unsubscribehostnamestatus_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."emailingDomain_unsubscribehostnamestatus_enum" AS ENUM (
    'PENDING',
    'ACTIVE',
    'FAILED'
);


--
-- Name: fieldMetadata_writability_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."fieldMetadata_writability_enum" AS ENUM (
    'OPEN',
    'APPLICATION',
    'SYSTEM'
);


--
-- Name: file_status_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core.file_status_enum AS ENUM (
    'PENDING',
    'UPLOADED',
    'REMOTE'
);


--
-- Name: indexMetadata_indextype_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."indexMetadata_indextype_enum" AS ENUM (
    'BTREE',
    'GIN'
);


--
-- Name: keyValuePair_type_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."keyValuePair_type_enum" AS ENUM (
    'USER_VARIABLE',
    'FEATURE_FLAG',
    'CONFIG_VARIABLE',
    'APPLICATION_VARIABLE'
);


--
-- Name: logicFunction_executionmode_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."logicFunction_executionmode_enum" AS ENUM (
    'LIVE',
    'PREBUILT'
);


--
-- Name: messageChannel_contactautocreationpolicy_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."messageChannel_contactautocreationpolicy_enum" AS ENUM (
    'SENT_AND_RECEIVED',
    'SENT',
    'NONE'
);


--
-- Name: messageChannel_messagefolderimportpolicy_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."messageChannel_messagefolderimportpolicy_enum" AS ENUM (
    'ALL_FOLDERS',
    'SELECTED_FOLDERS'
);


--
-- Name: messageChannel_pendinggroupemailsaction_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."messageChannel_pendinggroupemailsaction_enum" AS ENUM (
    'GROUP_EMAILS_DELETION',
    'GROUP_EMAILS_IMPORT',
    'NONE'
);


--
-- Name: messageChannel_syncstage_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."messageChannel_syncstage_enum" AS ENUM (
    'PENDING_CONFIGURATION',
    'MESSAGE_LIST_FETCH_PENDING',
    'MESSAGE_LIST_FETCH_SCHEDULED',
    'MESSAGE_LIST_FETCH_ONGOING',
    'MESSAGES_IMPORT_PENDING',
    'MESSAGES_IMPORT_SCHEDULED',
    'MESSAGES_IMPORT_ONGOING',
    'FAILED'
);


--
-- Name: messageChannel_syncstatus_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."messageChannel_syncstatus_enum" AS ENUM (
    'NOT_SYNCED',
    'ONGOING',
    'ACTIVE',
    'FAILED_INSUFFICIENT_PERMISSIONS',
    'FAILED_UNKNOWN'
);


--
-- Name: messageChannel_type_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."messageChannel_type_enum" AS ENUM (
    'EMAIL',
    'SMS',
    'EMAIL_GROUP',
    'APP'
);


--
-- Name: messageChannel_visibility_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."messageChannel_visibility_enum" AS ENUM (
    'METADATA',
    'SUBJECT',
    'SHARE_EVERYTHING'
);


--
-- Name: messageChannel_webhooksubscriptionstatus_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."messageChannel_webhooksubscriptionstatus_enum" AS ENUM (
    'PENDING',
    'ACTIVE',
    'FAILED',
    'EXPIRED'
);


--
-- Name: messageFolder_pendingsyncaction_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."messageFolder_pendingsyncaction_enum" AS ENUM (
    'FOLDER_DELETION',
    'FOLDER_IMPORT',
    'NONE'
);


--
-- Name: navigationMenuItem_type_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."navigationMenuItem_type_enum" AS ENUM (
    'VIEW',
    'FOLDER',
    'LINK',
    'OBJECT',
    'RECORD',
    'PAGE_LAYOUT',
    'AGENT_CHAT_CHANNEL',
    'NATIVE_PAGE',
    'WORKFLOW',
    'AGENT',
    'TAB',
    'SECTION'
);


--
-- Name: objectMetadata_openrecordin_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."objectMetadata_openrecordin_enum" AS ENUM (
    'SIDE_PANEL',
    'RECORD_PAGE',
    'USER_CHOICE'
);


--
-- Name: objectMetadata_readability_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."objectMetadata_readability_enum" AS ENUM (
    'OPEN',
    'PRIVATE',
    'INHERITED',
    'APPLICATION',
    'SYSTEM'
);


--
-- Name: objectMetadata_sharingreach_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."objectMetadata_sharingreach_enum" AS ENUM (
    'ROLE_ACCESS',
    'WORKSPACE'
);


--
-- Name: objectMetadata_writability_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."objectMetadata_writability_enum" AS ENUM (
    'OPEN',
    'APPLICATION',
    'SYSTEM'
);


--
-- Name: pageLayoutTab_layoutmode_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."pageLayoutTab_layoutmode_enum" AS ENUM (
    'GRID',
    'VERTICAL_LIST',
    'CANVAS'
);


--
-- Name: pageLayoutWidget_type_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."pageLayoutWidget_type_enum" AS ENUM (
    'VIEW',
    'IFRAME',
    'FIELD',
    'FIELDS',
    'GRAPH',
    'STANDALONE_RICH_TEXT',
    'TIMELINE',
    'TASKS',
    'NOTES',
    'FILES',
    'EMAILS',
    'CALENDAR',
    'FIELD_RICH_TEXT',
    'WORKFLOW',
    'WORKFLOW_VERSION',
    'WORKFLOW_RUN',
    'FRONT_COMPONENT',
    'RECORD_TABLE',
    'EMAIL_THREAD',
    'CALL_RECORDING_SUMMARY',
    'CALL_RECORDING_TRANSCRIPT',
    'MESSAGE_CAMPAIGN_BODY',
    'MESSAGE_CAMPAIGN_DETAILS',
    'FORM_FIELD',
    'CHAT_THREADS',
    'CHAT'
);


--
-- Name: pageLayout_type_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."pageLayout_type_enum" AS ENUM (
    'RECORD_INDEX',
    'RECORD_PAGE',
    'DASHBOARD',
    'STANDALONE_PAGE',
    'RECORD_FORM'
);


--
-- Name: routeTrigger_httpmethod_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."routeTrigger_httpmethod_enum" AS ENUM (
    'GET',
    'POST',
    'PUT',
    'PATCH',
    'DELETE'
);


--
-- Name: rowLevelPermissionPredicateGroup_logicaloperator_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."rowLevelPermissionPredicateGroup_logicaloperator_enum" AS ENUM (
    'AND',
    'OR'
);


--
-- Name: rowLevelPermissionPredicate_operand_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."rowLevelPermissionPredicate_operand_enum" AS ENUM (
    'IS',
    'IS_NOT_NULL',
    'IS_NOT',
    'LESS_THAN_OR_EQUAL',
    'GREATER_THAN_OR_EQUAL',
    'IS_BEFORE',
    'IS_AFTER',
    'CONTAINS',
    'DOES_NOT_CONTAIN',
    'IS_EMPTY',
    'IS_NOT_EMPTY',
    'IS_RELATIVE',
    'IS_IN_PAST',
    'IS_IN_FUTURE',
    'IS_TODAY',
    'VECTOR_SEARCH'
);


--
-- Name: settingsMenuItem_scope_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."settingsMenuItem_scope_enum" AS ENUM (
    'WORKSPACE',
    'USER'
);


--
-- Name: twoFactorAuthenticationMethod_status_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."twoFactorAuthenticationMethod_status_enum" AS ENUM (
    'PENDING',
    'VERIFIED'
);


--
-- Name: twoFactorAuthenticationMethod_strategy_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."twoFactorAuthenticationMethod_strategy_enum" AS ENUM (
    'TOTP'
);


--
-- Name: unsubscribeTopic_visibility_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."unsubscribeTopic_visibility_enum" AS ENUM (
    'PUBLIC',
    'PRIVATE'
);


--
-- Name: viewField_aggregateoperation_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."viewField_aggregateoperation_enum" AS ENUM (
    'MIN',
    'MAX',
    'AVG',
    'SUM',
    'COUNT',
    'COUNT_UNIQUE_VALUES',
    'COUNT_EMPTY',
    'COUNT_NOT_EMPTY',
    'COUNT_TRUE',
    'COUNT_FALSE',
    'PERCENTAGE_EMPTY',
    'PERCENTAGE_NOT_EMPTY'
);


--
-- Name: viewFilterGroup_logicaloperator_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."viewFilterGroup_logicaloperator_enum" AS ENUM (
    'AND',
    'OR',
    'NOT'
);


--
-- Name: viewFilter_operand_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."viewFilter_operand_enum" AS ENUM (
    'IS',
    'IS_NOT_NULL',
    'IS_NOT',
    'LESS_THAN_OR_EQUAL',
    'GREATER_THAN_OR_EQUAL',
    'IS_BEFORE',
    'IS_AFTER',
    'CONTAINS',
    'DOES_NOT_CONTAIN',
    'IS_EMPTY',
    'IS_NOT_EMPTY',
    'IS_RELATIVE',
    'IS_IN_PAST',
    'IS_IN_FUTURE',
    'IS_TODAY',
    'VECTOR_SEARCH'
);


--
-- Name: viewSort_direction_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."viewSort_direction_enum" AS ENUM (
    'ASC',
    'DESC'
);


--
-- Name: view_calendarlayout_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core.view_calendarlayout_enum AS ENUM (
    'DAY',
    'WEEK',
    'MONTH'
);


--
-- Name: view_kanbanaggregateoperation_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core.view_kanbanaggregateoperation_enum AS ENUM (
    'MIN',
    'MAX',
    'AVG',
    'SUM',
    'COUNT',
    'COUNT_UNIQUE_VALUES',
    'COUNT_EMPTY',
    'COUNT_NOT_EMPTY',
    'COUNT_TRUE',
    'COUNT_FALSE',
    'PERCENTAGE_EMPTY',
    'PERCENTAGE_NOT_EMPTY'
);


--
-- Name: view_key_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core.view_key_enum AS ENUM (
    'INDEX'
);


--
-- Name: view_openrecordin_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core.view_openrecordin_enum AS ENUM (
    'SIDE_PANEL',
    'RECORD_PAGE'
);


--
-- Name: view_type_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core.view_type_enum AS ENUM (
    'TABLE',
    'KANBAN',
    'CALENDAR',
    'LIST',
    'FIELDS_WIDGET',
    'TABLE_WIDGET',
    'KANBAN_WIDGET',
    'LIST_WIDGET',
    'CALENDAR_WIDGET'
);


--
-- Name: view_visibility_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core.view_visibility_enum AS ENUM (
    'WORKSPACE',
    'UNLISTED'
);


--
-- Name: workflowVersion_status_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."workflowVersion_status_enum" AS ENUM (
    'DRAFT',
    'ACTIVE',
    'DEACTIVATED',
    'ARCHIVED'
);


--
-- Name: workspaceSSOIdentityProvider_status_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."workspaceSSOIdentityProvider_status_enum" AS ENUM (
    'Active',
    'Inactive',
    'Error'
);


--
-- Name: workspaceSSOIdentityProvider_type_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."workspaceSSOIdentityProvider_type_enum" AS ENUM (
    'OIDC',
    'SAML'
);


--
-- Name: workspace_activationStatus_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core."workspace_activationStatus_enum" AS ENUM (
    'ONGOING_CREATION',
    'PENDING_CREATION',
    'CREATED',
    'ACTIVE',
    'INACTIVE',
    'SUSPENDED'
);


--
-- Name: workspace_discoverability_enum; Type: TYPE; Schema: core; Owner: -
--

CREATE TYPE core.workspace_discoverability_enum AS ENUM (
    'PUBLIC',
    'MEMBERS_AND_INVITEES',
    'HIDDEN'
);


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _typeorm_generated_columns_and_materialized_views; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core._typeorm_generated_columns_and_materialized_views (
    type character varying NOT NULL,
    database character varying,
    schema character varying,
    "table" character varying,
    name character varying,
    value text
);


--
-- Name: _typeorm_migrations; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core._typeorm_migrations (
    id integer NOT NULL,
    "timestamp" bigint NOT NULL,
    name character varying NOT NULL
);


--
-- Name: _typeorm_migrations_id_seq; Type: SEQUENCE; Schema: core; Owner: -
--

CREATE SEQUENCE core._typeorm_migrations_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: _typeorm_migrations_id_seq; Type: SEQUENCE OWNED BY; Schema: core; Owner: -
--

ALTER SEQUENCE core._typeorm_migrations_id_seq OWNED BY core._typeorm_migrations.id;


--
-- Name: agent; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core.agent (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    label character varying NOT NULL,
    icon character varying,
    description text,
    prompt text NOT NULL,
    "modelId" character varying DEFAULT 'workspace-default-model'::character varying NOT NULL,
    "responseFormat" jsonb DEFAULT '{"type": "text"}'::jsonb,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "applicationId" uuid NOT NULL,
    "modelConfiguration" jsonb,
    "universalIdentifier" uuid NOT NULL,
    "isSystem" boolean DEFAULT false NOT NULL,
    triggers jsonb DEFAULT '[]'::jsonb NOT NULL,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    "avatarColor" character varying,
    "avatarShape" character varying,
    "isPrivate" boolean DEFAULT false NOT NULL,
    "createdByUserWorkspaceId" uuid
);


--
-- Name: agentChatChannel; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."agentChatChannel" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    icon character varying,
    color character varying,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: agentChatChannelRole; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."agentChatChannelRole" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "channelId" uuid NOT NULL,
    "roleId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: apiKey; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."apiKey" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    "expiresAt" timestamp with time zone NOT NULL,
    "revokedAt" timestamp with time zone,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: appToken; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."appToken" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "userId" uuid,
    "workspaceId" uuid,
    type text DEFAULT 'REFRESH_TOKEN'::text NOT NULL,
    value text,
    "expiresAt" timestamp with time zone NOT NULL,
    "deletedAt" timestamp with time zone,
    "revokedAt" timestamp with time zone,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    context jsonb
);


--
-- Name: application; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core.application (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    name text NOT NULL,
    description text,
    version text,
    "sourceType" text DEFAULT 'local'::text NOT NULL,
    "sourcePath" text NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "logicFunctionLayerId" uuid,
    "canBeUninstalled" boolean DEFAULT true NOT NULL,
    "defaultRoleId" uuid,
    "packageJsonChecksum" text,
    "packageJsonFileId" uuid,
    "yarnLockChecksum" text,
    "yarnLockFileId" uuid,
    "availablePackages" jsonb DEFAULT '{}'::jsonb NOT NULL,
    "settingsCustomTabFrontComponentId" uuid,
    "applicationRegistrationId" uuid,
    "isSdkLayerStale" boolean DEFAULT false NOT NULL,
    logo text,
    "primaryPublicDomainId" uuid,
    "logoFileId" uuid,
    "autoUpgrade" boolean DEFAULT false NOT NULL,
    "sdkClientCoreChecksum" text,
    "frontComponentSharedDependenciesChecksum" text,
    "frontComponentSharedDependenciesBuiltPath" text,
    "uninstallLogicFunctionId" uuid,
    "uninstallHookCompletedForRequestedAt" timestamp with time zone,
    state text DEFAULT 'INSTALLED'::text NOT NULL,
    billing jsonb DEFAULT '{}'::jsonb NOT NULL,
    "healthCheckLogicFunctionId" uuid,
    "grantedCapabilities" character varying[] DEFAULT '{}'::character varying[] NOT NULL,
    "isUserHealthCheckEnabled" boolean DEFAULT false NOT NULL,
    "grantedIframeMediaPermissions" jsonb DEFAULT '[]'::jsonb NOT NULL,
    "requestedRoleId" uuid,
    "acceptedBillingPrices" jsonb
);


--
-- Name: applicationAuthorization; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."applicationAuthorization" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "userId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "userWorkspaceId" uuid NOT NULL,
    scopes text[],
    "lastAuthorizedAt" timestamp with time zone,
    "lastUsedAt" timestamp with time zone NOT NULL,
    "revokedAt" timestamp with time zone,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: applicationRegistration; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."applicationRegistration" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    name text NOT NULL,
    "oAuthClientId" text NOT NULL,
    "oAuthClientSecretHash" text,
    "oAuthRedirectUris" text[] DEFAULT '{}'::text[] NOT NULL,
    "oAuthScopes" text[] DEFAULT '{}'::text[] NOT NULL,
    "createdByUserId" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "workspaceId" uuid,
    "sourceType" text DEFAULT 'local'::text NOT NULL,
    "sourcePackage" text,
    "tarballFileId" uuid,
    "latestAvailableVersion" text,
    "isVetted" boolean DEFAULT false NOT NULL,
    manifest jsonb,
    "isListed" boolean DEFAULT false NOT NULL,
    "isPreInstalled" boolean DEFAULT false NOT NULL,
    logo text,
    description text,
    author text,
    category text,
    "websiteUrl" text,
    "aboutDescription" text,
    "termsUrl" text,
    "emailSupport" text,
    "issueReportUrl" text,
    screenshots text[] DEFAULT '{}'::text[] NOT NULL,
    "galleryImages" jsonb,
    "logoFileId" uuid,
    "pricingDescription" text,
    CONSTRAINT "CHK_NPM_HAS_SOURCE_PACKAGE" CHECK ((("sourceType" <> 'npm'::text) OR ("sourcePackage" IS NOT NULL)))
);


--
-- Name: applicationRegistrationVariable; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."applicationRegistrationVariable" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    key text NOT NULL,
    "encryptedValue" text NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    "isSecret" boolean DEFAULT true NOT NULL,
    "isRequired" boolean DEFAULT false NOT NULL,
    "applicationRegistrationId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    type text DEFAULT 'TEXT'::text NOT NULL,
    options jsonb,
    "isDeprecated" boolean DEFAULT false NOT NULL,
    CONSTRAINT "CHK_applicationRegistrationVariable_deprecated_not_required" CHECK ((NOT ("isRequired" AND "isDeprecated"))),
    CONSTRAINT "CHK_applicationRegistrationVariable_encryptedValue_encrypted" CHECK (("encryptedValue" ~~ 'enc:v2:%'::text))
);


--
-- Name: applicationRegistrationVersion; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."applicationRegistrationVersion" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "applicationRegistrationId" uuid NOT NULL,
    version character varying NOT NULL,
    "sourceType" character varying NOT NULL,
    "sourcePackage" character varying,
    "tarballFileId" uuid,
    "tarballUrl" character varying,
    "publishedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT "CHK_APP_REG_VERSION_NPM_HAS_SOURCE_PACKAGE" CHECK (((("sourceType")::text <> 'npm'::text) OR ("sourcePackage" IS NOT NULL)))
);


--
-- Name: applicationTranslation; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."applicationTranslation" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "applicationRegistrationId" uuid,
    locale text NOT NULL,
    messages jsonb DEFAULT '{}'::jsonb NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone
);


--
-- Name: applicationVariable; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."applicationVariable" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    key text NOT NULL,
    value text,
    description text DEFAULT ''::text NOT NULL,
    "isSecret" boolean DEFAULT false NOT NULL,
    "applicationId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    type text DEFAULT 'TEXT'::text NOT NULL,
    options jsonb,
    "isDeprecated" boolean DEFAULT false NOT NULL,
    label text DEFAULT ''::text NOT NULL,
    "isRequired" boolean DEFAULT false NOT NULL,
    scope core."applicationVariable_scope_enum" DEFAULT 'WORKSPACE'::core."applicationVariable_scope_enum" NOT NULL,
    "defaultValue" text,
    CONSTRAINT "CHK_applicationVariable_default_value_not_secret" CHECK ((NOT ("isSecret" AND ("defaultValue" IS NOT NULL)))),
    CONSTRAINT "CHK_applicationVariable_deprecated_not_required" CHECK ((NOT ("isRequired" AND "isDeprecated"))),
    CONSTRAINT "CHK_applicationVariable_value_encrypted" CHECK ((value ~~ 'enc:v2:%'::text)),
    CONSTRAINT "CHK_applicationVariable_value_null_only_for_user_scope" CHECK (((scope = 'USER'::core."applicationVariable_scope_enum") = (value IS NULL)))
);


--
-- Name: approvedAccessDomain; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."approvedAccessDomain" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    domain character varying NOT NULL,
    "isValidated" boolean DEFAULT false NOT NULL,
    "workspaceId" uuid NOT NULL
);


--
-- Name: calendarChannel; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."calendarChannel" (
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    handle character varying NOT NULL,
    "syncStatus" core."calendarChannel_syncstatus_enum" DEFAULT 'NOT_SYNCED'::core."calendarChannel_syncstatus_enum" NOT NULL,
    "syncStage" core."calendarChannel_syncstage_enum" NOT NULL,
    visibility core."calendarChannel_visibility_enum" NOT NULL,
    "isContactAutoCreationEnabled" boolean DEFAULT true NOT NULL,
    "contactAutoCreationPolicy" core."calendarChannel_contactautocreationpolicy_enum" DEFAULT 'AS_PARTICIPANT_AND_ORGANIZER'::core."calendarChannel_contactautocreationpolicy_enum" NOT NULL,
    "isSyncEnabled" boolean DEFAULT true NOT NULL,
    "syncCursor" character varying,
    "syncedAt" timestamp with time zone,
    "syncStageStartedAt" timestamp with time zone,
    "throttleFailureCount" integer DEFAULT 0 NOT NULL,
    "connectedAccountId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "webhookSubscriptionExternalId" character varying,
    "webhookSubscriptionExternalResourceId" character varying,
    "webhookSubscriptionClientState" character varying,
    "webhookSubscriptionStatus" core."calendarChannel_webhooksubscriptionstatus_enum",
    "webhookSubscriptionExpiresAt" timestamp with time zone,
    "excludePrivateEvents" boolean DEFAULT false NOT NULL
);


--
-- Name: commandMenuItem; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."commandMenuItem" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "workflowVersionId" uuid,
    label character varying NOT NULL,
    icon character varying,
    "isPinned" boolean DEFAULT false NOT NULL,
    "availabilityType" core."commandMenuItem_availabilitytype_enum" DEFAULT 'GLOBAL'::core."commandMenuItem_availabilitytype_enum" NOT NULL,
    "availabilityObjectMetadataId" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "frontComponentId" uuid,
    "conditionalAvailabilityExpression" character varying,
    "shortLabel" character varying,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "engineComponentKey" character varying NOT NULL,
    "hotKeys" text[],
    payload jsonb,
    "pageLayoutId" uuid,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL,
    "navigationTargetObjectMetadataId" uuid,
    "conditionalPinnedExpression" character varying,
    "coreWorkflowVersionId" uuid,
    "availabilityCoreObjectName" character varying,
    CONSTRAINT "CHK_CMD_MENU_ITEM_ENGINE_KEY_COHERENCE" CHECK ((((("engineComponentKey")::text = 'TRIGGER_WORKFLOW_VERSION'::text) AND (("workflowVersionId" IS NOT NULL) OR ("coreWorkflowVersionId" IS NOT NULL)) AND ("frontComponentId" IS NULL) AND (payload IS NULL) AND ("navigationTargetObjectMetadataId" IS NULL)) OR ((("engineComponentKey")::text = 'FRONT_COMPONENT_RENDERER'::text) AND ("frontComponentId" IS NOT NULL) AND ("workflowVersionId" IS NULL) AND ("coreWorkflowVersionId" IS NULL) AND (payload IS NULL) AND ("navigationTargetObjectMetadataId" IS NULL)) OR ((("engineComponentKey")::text = 'NAVIGATION'::text) AND (((payload IS NOT NULL) AND ("navigationTargetObjectMetadataId" IS NULL)) OR ((payload IS NULL) AND ("navigationTargetObjectMetadataId" IS NOT NULL))) AND ("workflowVersionId" IS NULL) AND ("coreWorkflowVersionId" IS NULL) AND ("frontComponentId" IS NULL)) OR ((("engineComponentKey")::text <> ALL ((ARRAY['TRIGGER_WORKFLOW_VERSION'::character varying, 'FRONT_COMPONENT_RENDERER'::character varying, 'NAVIGATION'::character varying])::text[])) AND ("workflowVersionId" IS NULL) AND ("coreWorkflowVersionId" IS NULL) AND ("frontComponentId" IS NULL) AND (payload IS NULL) AND ("navigationTargetObjectMetadataId" IS NULL))))
);


--
-- Name: connectedAccount; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."connectedAccount" (
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    handle character varying NOT NULL,
    provider character varying NOT NULL,
    "accessToken" character varying,
    "refreshToken" character varying,
    "lastCredentialsRefreshedAt" timestamp with time zone,
    "authFailedAt" timestamp with time zone,
    "handleAliases" character varying[],
    scopes character varying[],
    "connectionParameters" jsonb,
    "lastSignedInAt" timestamp with time zone,
    "oidcTokenClaims" jsonb,
    "userWorkspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "connectionProviderId" uuid,
    "applicationId" uuid,
    name character varying,
    "archivedAt" timestamp with time zone,
    "authFailedReason" character varying,
    "externalAccountId" character varying(512),
    scope character varying DEFAULT 'USER'::character varying NOT NULL,
    "accessTokenExpiresAt" timestamp with time zone,
    CONSTRAINT "CHK_connectedAccount_accessToken_encrypted" CHECK ((("accessToken" IS NULL) OR (("accessToken")::text ~~ 'enc:v2:%'::text))),
    CONSTRAINT "CHK_connectedAccount_connectionParameters_encrypted" CHECK ((("connectionParameters" IS NULL) OR ((((("connectionParameters" -> 'IMAP'::text) ->> 'password'::text) IS NULL) OR ((("connectionParameters" -> 'IMAP'::text) ->> 'password'::text) ~~ 'enc:v2:%'::text)) AND (((("connectionParameters" -> 'SMTP'::text) ->> 'password'::text) IS NULL) OR ((("connectionParameters" -> 'SMTP'::text) ->> 'password'::text) ~~ 'enc:v2:%'::text)) AND (((("connectionParameters" -> 'CALDAV'::text) ->> 'password'::text) IS NULL) OR ((("connectionParameters" -> 'CALDAV'::text) ->> 'password'::text) ~~ 'enc:v2:%'::text))))),
    CONSTRAINT "CHK_connectedAccount_refreshToken_encrypted" CHECK ((("refreshToken" IS NULL) OR (("refreshToken")::text ~~ 'enc:v2:%'::text)))
);


--
-- Name: connectionProvider; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."connectionProvider" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "workspaceId" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    name character varying NOT NULL,
    "displayName" character varying NOT NULL,
    type character varying NOT NULL,
    "oauthConfig" jsonb,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "onConnectLogicFunctionUniversalIdentifier" uuid,
    "onDisconnectLogicFunctionUniversalIdentifier" uuid,
    logo character varying,
    "identifyAccountLogicFunctionUniversalIdentifier" uuid,
    scope core."connectionProvider_scope_enum" DEFAULT 'WORKSPACE'::core."connectionProvider_scope_enum" NOT NULL,
    "customConfig" jsonb,
    "oauthDiscoveryConfig" jsonb
);


--
-- Name: connectionProviderOAuthClient; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."connectionProviderOAuthClient" (
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "connectionProviderId" uuid NOT NULL,
    "registrationMethod" character varying NOT NULL,
    "clientId" character varying NOT NULL,
    "clientSecret" character varying,
    "clientSecretExpiresAt" timestamp with time zone,
    "tokenEndpointAuthMethod" character varying,
    "redirectUri" character varying NOT NULL,
    "discoveryState" jsonb NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT "CHK_connectionProviderOAuthClient_clientSecret_encrypted" CHECK ((("clientSecret" IS NULL) OR (("clientSecret")::text ~~ 'enc:v2:%'::text)))
);


--
-- Name: dataSource; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."dataSource" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    label character varying,
    url character varying,
    schema character varying,
    type core."dataSource_type_enum" DEFAULT 'postgres'::core."dataSource_type_enum" NOT NULL,
    "isRemote" boolean DEFAULT false NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: deferredWorkspaceOperation; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."deferredWorkspaceOperation" (
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "applicationUniversalIdentifier" character varying NOT NULL,
    name character varying NOT NULL,
    payload jsonb NOT NULL,
    "position" integer NOT NULL,
    "runByVersion" character varying,
    status character varying DEFAULT 'PENDING'::character varying NOT NULL,
    attempts integer DEFAULT 0 NOT NULL,
    "lastError" text,
    "startedAt" timestamp with time zone,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: deferredWorkspaceMigrationAction; Type: VIEW; Schema: core; Owner: -
--

CREATE VIEW core."deferredWorkspaceMigrationAction" AS
 SELECT "workspaceId",
    id,
    "applicationUniversalIdentifier",
    name,
    payload,
    "position",
    "runByVersion",
    status,
    attempts,
    "lastError",
    "startedAt",
    "createdAt",
    "updatedAt"
   FROM core."deferredWorkspaceOperation";


--
-- Name: dpaAgreement; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."dpaAgreement" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    type core."dpaAgreement_type_enum" NOT NULL,
    "templateVersion" character varying NOT NULL,
    region character varying NOT NULL,
    "processorEntity" character varying NOT NULL,
    "customerLegalEntityName" character varying,
    "signatoryName" character varying,
    "signatoryTitle" character varying,
    "signedFileId" uuid,
    "acceptedByUserId" uuid,
    "acceptedByEmail" character varying,
    "acceptedAt" timestamp with time zone NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "workspaceId" uuid NOT NULL
);


--
-- Name: emailingDomain; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."emailingDomain" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    domain character varying NOT NULL,
    status core."emailingDomain_status_enum" DEFAULT 'PENDING'::core."emailingDomain_status_enum" NOT NULL,
    "verificationRecords" jsonb,
    "verifiedAt" timestamp with time zone,
    "workspaceId" uuid NOT NULL,
    "tenantStatus" core."emailingDomain_tenantstatus_enum" DEFAULT 'ACTIVE'::core."emailingDomain_tenantstatus_enum" NOT NULL,
    "unsubscribeHostname" character varying,
    "unsubscribeHostnameId" character varying,
    "unsubscribeHostnameStatus" core."emailingDomain_unsubscribehostnamestatus_enum"
);


--
-- Name: featureFlag; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."featureFlag" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    key text NOT NULL,
    "workspaceId" uuid NOT NULL,
    value boolean NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: fieldMetadata; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."fieldMetadata" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    type character varying NOT NULL,
    name character varying NOT NULL,
    label character varying NOT NULL,
    "defaultValue" jsonb,
    description text,
    icon character varying,
    options jsonb,
    settings jsonb,
    "isActive" boolean DEFAULT false NOT NULL,
    "isSystem" boolean DEFAULT false NOT NULL,
    "isUIReadOnly" boolean DEFAULT false NOT NULL,
    "isNullable" boolean DEFAULT true,
    "workspaceId" uuid NOT NULL,
    "isLabelSyncedWithName" boolean DEFAULT false NOT NULL,
    "relationTargetFieldMetadataId" uuid,
    "relationTargetObjectMetadataId" uuid,
    "morphId" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "isUIEditable" boolean DEFAULT true NOT NULL,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL,
    overrides jsonb,
    writability core."fieldMetadata_writability_enum" DEFAULT 'OPEN'::core."fieldMetadata_writability_enum" NOT NULL,
    "isAuditLogged" boolean DEFAULT true NOT NULL,
    "disabledFieldPermissionActions" jsonb,
    CONSTRAINT "CHK_FIELD_METADATA_MORPH_RELATION_REQUIRES_MORPH_ID" CHECK ((((type)::text <> 'MORPH_RELATION'::text) OR (((type)::text = 'MORPH_RELATION'::text) AND ("morphId" IS NOT NULL))))
);


--
-- Name: fieldMetadataOption; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."fieldMetadataOption" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "fieldMetadataId" uuid NOT NULL,
    value character varying NOT NULL,
    label character varying NOT NULL,
    color character varying,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: fieldPermission; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."fieldPermission" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "roleId" uuid NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    "fieldMetadataId" uuid NOT NULL,
    "canReadFieldValue" boolean,
    "canUpdateFieldValue" boolean,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "rowLevelPermissionPredicateGroupId" uuid,
    "rowLevelPermissionRuleId" uuid
);


--
-- Name: file; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core.file (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    size bigint NOT NULL,
    "workspaceId" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "applicationId" uuid,
    path character varying NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "isStaticAsset" boolean DEFAULT false NOT NULL,
    settings jsonb,
    "mimeType" character varying DEFAULT 'application/octet-stream'::character varying NOT NULL,
    status core.file_status_enum DEFAULT 'UPLOADED'::core.file_status_enum NOT NULL,
    "applicationRegistrationId" uuid,
    "contentSha256" character varying,
    "remoteOrigin" jsonb,
    CONSTRAINT "CHK_FILE_WORKSPACE_ID_OR_APPLICATION_REGISTRATION_ID" CHECK ((("workspaceId" IS NOT NULL) OR ("applicationRegistrationId" IS NOT NULL))),
    CONSTRAINT "CHK_FILE_WORKSPACE_ID_XOR_APPLICATION_REGISTRATION_ID" CHECK ((("workspaceId" IS NULL) OR ("applicationRegistrationId" IS NULL)))
);


--
-- Name: frontComponent; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."frontComponent" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    description character varying,
    "sourceComponentPath" character varying NOT NULL,
    "builtComponentPath" character varying NOT NULL,
    "componentName" character varying NOT NULL,
    "builtComponentChecksum" character varying NOT NULL,
    "isHeadless" boolean DEFAULT false NOT NULL,
    "usesSdkClient" boolean DEFAULT false NOT NULL,
    "widgetAvailability" jsonb DEFAULT '[]'::jsonb NOT NULL
);


--
-- Name: indexFieldMetadata; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."indexFieldMetadata" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "indexMetadataId" uuid NOT NULL,
    "fieldMetadataId" uuid NOT NULL,
    "order" integer NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "workspaceId" uuid NOT NULL,
    "subFieldName" text
);


--
-- Name: indexMetadata; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."indexMetadata" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    name character varying NOT NULL,
    "workspaceId" uuid NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    "isUnique" boolean DEFAULT false NOT NULL,
    "indexWhereClause" text,
    "indexType" core."indexMetadata_indextype_enum" DEFAULT 'BTREE'::core."indexMetadata_indextype_enum" NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL
);


--
-- Name: keyValuePair; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."keyValuePair" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "userId" uuid,
    "workspaceId" uuid,
    key text NOT NULL,
    value jsonb,
    "textValueDeprecated" text,
    type core."keyValuePair_type_enum" DEFAULT 'USER_VARIABLE'::core."keyValuePair_type_enum" NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "applicationId" uuid
);


--
-- Name: logicFunction; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."logicFunction" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    description character varying,
    runtime character varying DEFAULT 'nodejs22.x'::character varying NOT NULL,
    "timeoutSeconds" integer DEFAULT 300 NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    checksum text,
    "sourceHandlerPath" character varying NOT NULL,
    "handlerName" character varying NOT NULL,
    "builtHandlerPath" character varying NOT NULL,
    "cronTriggerSettings" jsonb,
    "databaseEventTriggerSettings" jsonb,
    "httpRouteTriggerSettings" jsonb,
    "isBuildUpToDate" boolean DEFAULT true NOT NULL,
    "toolTriggerSettings" jsonb,
    "workflowActionTriggerSettings" jsonb,
    "executionMode" core."logicFunction_executionmode_enum" DEFAULT 'LIVE'::core."logicFunction_executionmode_enum" NOT NULL,
    "serverRouteTriggerSettings" jsonb,
    CONSTRAINT "CHK_349d2959a97c0b14fa0bf7cadd" CHECK ((("timeoutSeconds" >= 1) AND ("timeoutSeconds" <= 900)))
);


--
-- Name: logicFunctionLayer; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."logicFunctionLayer" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "packageJson" jsonb NOT NULL,
    "yarnLock" text NOT NULL,
    "yarnLockChecksum" text NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "packageJsonChecksum" text,
    "availablePackages" jsonb DEFAULT '{}'::jsonb NOT NULL
);


--
-- Name: mcpServer; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."mcpServer" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    label character varying NOT NULL,
    url character varying NOT NULL,
    "connectionProviderId" uuid,
    "isActive" boolean DEFAULT true NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    tools jsonb DEFAULT '[]'::jsonb NOT NULL,
    "toolsSyncedAt" timestamp with time zone,
    "toolsSyncError" character varying
);


--
-- Name: messageChannel; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."messageChannel" (
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    visibility core."messageChannel_visibility_enum" NOT NULL,
    handle character varying NOT NULL,
    type core."messageChannel_type_enum" NOT NULL,
    "isContactAutoCreationEnabled" boolean DEFAULT true NOT NULL,
    "contactAutoCreationPolicy" core."messageChannel_contactautocreationpolicy_enum" DEFAULT 'SENT'::core."messageChannel_contactautocreationpolicy_enum" NOT NULL,
    "messageFolderImportPolicy" core."messageChannel_messagefolderimportpolicy_enum" DEFAULT 'ALL_FOLDERS'::core."messageChannel_messagefolderimportpolicy_enum" NOT NULL,
    "excludeNonProfessionalEmails" boolean DEFAULT true NOT NULL,
    "excludeGroupEmails" boolean DEFAULT true NOT NULL,
    "pendingGroupEmailsAction" core."messageChannel_pendinggroupemailsaction_enum" NOT NULL,
    "isSyncEnabled" boolean DEFAULT true NOT NULL,
    "syncCursor" character varying,
    "syncedAt" timestamp with time zone,
    "syncStatus" core."messageChannel_syncstatus_enum" DEFAULT 'NOT_SYNCED'::core."messageChannel_syncstatus_enum" NOT NULL,
    "syncStage" core."messageChannel_syncstage_enum" NOT NULL,
    "syncStageStartedAt" timestamp with time zone,
    "throttleFailureCount" integer DEFAULT 0 NOT NULL,
    "throttleRetryAfter" timestamp with time zone,
    "connectedAccountId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "webhookSubscriptionExternalId" character varying,
    "webhookSubscriptionClientState" character varying,
    "webhookSubscriptionStatus" core."messageChannel_webhooksubscriptionstatus_enum",
    "webhookSubscriptionExpiresAt" timestamp with time zone,
    "displayName" character varying,
    "participantHandleFieldUniversalIdentifier" uuid,
    "messagingProviderUniversalIdentifier" uuid
);


--
-- Name: messageFolder; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."messageFolder" (
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying,
    "syncCursor" character varying,
    "isSentFolder" boolean NOT NULL,
    "isSynced" boolean NOT NULL,
    "parentFolderId" character varying,
    "externalId" character varying,
    "pendingSyncAction" core."messageFolder_pendingsyncaction_enum" DEFAULT 'NONE'::core."messageFolder_pendingsyncaction_enum" NOT NULL,
    "messageChannelId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: messagingProvider; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."messagingProvider" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    "displayName" character varying NOT NULL,
    "connectionProviderUniversalIdentifier" uuid NOT NULL,
    "sendMessageLogicFunctionUniversalIdentifier" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "conversationStyle" character varying,
    "markThreadAsReadLogicFunctionUniversalIdentifier" uuid
);


--
-- Name: navigationMenuItem; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."navigationMenuItem" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "userWorkspaceId" uuid,
    "targetRecordId" uuid,
    "targetObjectMetadataId" uuid,
    "viewId" uuid,
    name text,
    "parentId" uuid,
    "position" double precision NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    link text,
    color text,
    icon text,
    type core."navigationMenuItem_type_enum" NOT NULL,
    "pageLayoutId" uuid,
    "agentChatChannelId" uuid,
    "nativePageKey" text,
    "workflowId" uuid,
    "agentId" uuid,
    "builtInSectionKey" text,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    CONSTRAINT "CHK_navigation_menu_item_type_fields" CHECK (((type = 'FOLDER'::core."navigationMenuItem_type_enum") OR ((type = 'OBJECT'::core."navigationMenuItem_type_enum") AND ("targetObjectMetadataId" IS NOT NULL)) OR ((type = 'VIEW'::core."navigationMenuItem_type_enum") AND ("viewId" IS NOT NULL)) OR ((type = 'RECORD'::core."navigationMenuItem_type_enum") AND ("targetRecordId" IS NOT NULL) AND ("targetObjectMetadataId" IS NOT NULL)) OR ((type = 'LINK'::core."navigationMenuItem_type_enum") AND (link IS NOT NULL)) OR ((type = 'PAGE_LAYOUT'::core."navigationMenuItem_type_enum") AND ("pageLayoutId" IS NOT NULL)) OR ((type = 'AGENT_CHAT_CHANNEL'::core."navigationMenuItem_type_enum") AND ("agentChatChannelId" IS NOT NULL)) OR ((type = 'NATIVE_PAGE'::core."navigationMenuItem_type_enum") AND ("nativePageKey" IS NOT NULL)) OR ((type = 'WORKFLOW'::core."navigationMenuItem_type_enum") AND ("workflowId" IS NOT NULL)) OR ((type = 'AGENT'::core."navigationMenuItem_type_enum") AND ("agentId" IS NOT NULL)) OR (type = 'TAB'::core."navigationMenuItem_type_enum") OR (type = 'SECTION'::core."navigationMenuItem_type_enum")))
);


--
-- Name: objectMetadata; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."objectMetadata" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "dataSourceId" uuid,
    "nameSingular" character varying NOT NULL,
    "namePlural" character varying NOT NULL,
    "labelSingular" character varying NOT NULL,
    "labelPlural" character varying NOT NULL,
    description text,
    icon character varying,
    "targetTableName" character varying NOT NULL,
    "isRemote" boolean DEFAULT false NOT NULL,
    "isActive" boolean DEFAULT false NOT NULL,
    "isSystem" boolean DEFAULT false NOT NULL,
    "isUIReadOnly" boolean DEFAULT false NOT NULL,
    "isAuditLogged" boolean DEFAULT true NOT NULL,
    "isSearchable" boolean DEFAULT false NOT NULL,
    "duplicateCriteria" jsonb,
    shortcut character varying,
    "labelIdentifierFieldMetadataId" uuid,
    "imageIdentifierFieldMetadataId" uuid,
    "isLabelSyncedWithName" boolean DEFAULT false NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "applicationId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    color text,
    "isUIEditable" boolean DEFAULT true NOT NULL,
    "isUICreatable" boolean DEFAULT true NOT NULL,
    overrides jsonb,
    "openRecordIn" core."objectMetadata_openrecordin_enum" DEFAULT 'USER_CHOICE'::core."objectMetadata_openrecordin_enum" NOT NULL,
    writability core."objectMetadata_writability_enum" DEFAULT 'OPEN'::core."objectMetadata_writability_enum" NOT NULL,
    readability core."objectMetadata_readability_enum" DEFAULT 'OPEN'::core."objectMetadata_readability_enum" NOT NULL,
    "readabilityParentFieldUniversalIdentifiers" uuid[],
    "sharingReach" core."objectMetadata_sharingreach_enum" DEFAULT 'WORKSPACE'::core."objectMetadata_sharingreach_enum" NOT NULL,
    "disabledObjectPermissionActions" jsonb
);


--
-- Name: objectPermission; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."objectPermission" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "roleId" uuid NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    "canReadObjectRecords" boolean,
    "canUpdateObjectRecords" boolean,
    "canSoftDeleteObjectRecords" boolean,
    "canDestroyObjectRecords" boolean,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "canCreateObjectRecords" boolean
);


--
-- Name: pageLayout; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."pageLayout" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    "workspaceId" uuid NOT NULL,
    type core."pageLayout_type_enum" DEFAULT 'RECORD_PAGE'::core."pageLayout_type_enum" NOT NULL,
    "objectMetadataId" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "defaultTabToFocusOnMobileAndSidePanelId" uuid,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL,
    "isFirstTabPinned" boolean DEFAULT true NOT NULL
);


--
-- Name: pageLayoutTab; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."pageLayoutTab" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    title character varying NOT NULL,
    "workspaceId" uuid NOT NULL,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "pageLayoutId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    icon character varying,
    "layoutMode" core."pageLayoutTab_layoutmode_enum" DEFAULT 'GRID'::core."pageLayoutTab_layoutmode_enum" NOT NULL,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL
);


--
-- Name: pageLayoutWidget; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."pageLayoutWidget" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "pageLayoutTabId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    title character varying NOT NULL,
    type core."pageLayoutWidget_type_enum" DEFAULT 'VIEW'::core."pageLayoutWidget_type_enum" NOT NULL,
    "objectMetadataId" uuid,
    "gridPosition" jsonb NOT NULL,
    configuration jsonb NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "conditionalDisplay" jsonb,
    "position" jsonb,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    "conditionalAvailabilityExpression" character varying,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL
);


--
-- Name: pendingWakeUp; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."pendingWakeUp" (
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "ownerType" character varying NOT NULL,
    "ownerId" uuid NOT NULL,
    "ownerKey" character varying NOT NULL,
    condition jsonb NOT NULL,
    "eventName" character varying,
    "resumeAt" timestamp with time zone,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    payload jsonb
);


--
-- Name: permissionFlag; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."permissionFlag" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "workspaceId" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    key character varying NOT NULL,
    label character varying NOT NULL,
    description text,
    icon character varying,
    "permissionType" character varying NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: publicDomain; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."publicDomain" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    domain character varying NOT NULL,
    "isValidated" boolean DEFAULT false NOT NULL,
    "workspaceId" uuid NOT NULL,
    "applicationId" uuid NOT NULL
);


--
-- Name: relatedRecordPath; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."relatedRecordPath" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    "firstFieldMetadataId" uuid NOT NULL,
    "secondFieldMetadataId" uuid,
    "thirdFieldMetadataId" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "isInherent" boolean DEFAULT false NOT NULL
);


--
-- Name: role; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core.role (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    label character varying NOT NULL,
    "canUpdateAllSettings" boolean DEFAULT false NOT NULL,
    "canAccessAllTools" boolean DEFAULT false NOT NULL,
    "canReadAllObjectRecords" boolean DEFAULT false NOT NULL,
    "canUpdateAllObjectRecords" boolean DEFAULT false NOT NULL,
    "canSoftDeleteAllObjectRecords" boolean DEFAULT false NOT NULL,
    "canDestroyAllObjectRecords" boolean DEFAULT false NOT NULL,
    description text,
    icon character varying,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "isEditable" boolean DEFAULT true NOT NULL,
    "canBeAssignedToUsers" boolean DEFAULT true NOT NULL,
    "canBeAssignedToAgents" boolean DEFAULT true NOT NULL,
    "canBeAssignedToApiKeys" boolean DEFAULT true NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "canAccessAllAgentChatChannels" boolean DEFAULT false NOT NULL,
    "canBeAssignedToApplications" boolean DEFAULT false NOT NULL,
    "canCreateAllObjectRecords" boolean DEFAULT false NOT NULL,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL
);


--
-- Name: rolePermissionFlag; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."rolePermissionFlag" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "roleId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "permissionFlagId" uuid NOT NULL
);


--
-- Name: roleTarget; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."roleTarget" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "workspaceId" uuid NOT NULL,
    "roleId" uuid NOT NULL,
    "userWorkspaceId" uuid,
    "agentId" uuid,
    "apiKeyId" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "webhookId" uuid,
    "targetApplicationId" uuid,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL,
    CONSTRAINT "CHK_role_target_single_entity" CHECK (((((((("agentId" IS NOT NULL))::integer + (("userWorkspaceId" IS NOT NULL))::integer) + (("apiKeyId" IS NOT NULL))::integer) + (("webhookId" IS NOT NULL))::integer) + (("targetApplicationId" IS NOT NULL))::integer) = 1))
);


--
-- Name: rowLevelPermissionPredicate; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."rowLevelPermissionPredicate" (
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "fieldMetadataId" uuid NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    operand core."rowLevelPermissionPredicate_operand_enum" DEFAULT 'CONTAINS'::core."rowLevelPermissionPredicate_operand_enum" NOT NULL,
    value jsonb,
    "subFieldName" text,
    "workspaceMemberFieldMetadataId" uuid,
    "workspaceMemberSubFieldName" text,
    "rowLevelPermissionPredicateGroupId" uuid,
    "positionInRowLevelPermissionPredicateGroup" double precision,
    "workspaceId" uuid NOT NULL,
    "roleId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "relationFieldMetadataId" uuid,
    "rowLevelPermissionRuleId" uuid,
    "workspaceMemberRelationFieldMetadataId" uuid
);


--
-- Name: rowLevelPermissionPredicateGroup; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."rowLevelPermissionPredicateGroup" (
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "parentRowLevelPermissionPredicateGroupId" uuid,
    "logicalOperator" core."rowLevelPermissionPredicateGroup_logicaloperator_enum" DEFAULT 'AND'::core."rowLevelPermissionPredicateGroup_logicaloperator_enum" NOT NULL,
    "positionInRowLevelPermissionPredicateGroup" double precision,
    "workspaceId" uuid NOT NULL,
    "roleId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "objectMetadataId" uuid NOT NULL,
    "canReadRecords" boolean DEFAULT false NOT NULL,
    "canUpdateRecords" boolean DEFAULT false NOT NULL,
    "canSoftDeleteRecords" boolean DEFAULT false NOT NULL,
    "canDestroyRecords" boolean DEFAULT false NOT NULL,
    "canCreateRecords" boolean DEFAULT false NOT NULL,
    "rowLevelPermissionRuleId" uuid
);


--
-- Name: rowLevelPermissionRule; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."rowLevelPermissionRule" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "roleId" uuid NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    "canReadRecords" boolean DEFAULT false NOT NULL,
    "canCreateRecords" boolean DEFAULT false NOT NULL,
    "canUpdateRecords" boolean DEFAULT false NOT NULL,
    "canSoftDeleteRecords" boolean DEFAULT false NOT NULL,
    "canDestroyRecords" boolean DEFAULT false NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: searchFieldMetadata; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."searchFieldMetadata" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    "fieldMetadataId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "position" double precision NOT NULL,
    "tsVectorFieldMetadataId" uuid NOT NULL,
    "isSystemSideEffect" boolean DEFAULT true NOT NULL
);


--
-- Name: settingsMenuItem; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."settingsMenuItem" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "frontComponentId" uuid NOT NULL,
    title character varying NOT NULL,
    icon character varying,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    scope core."settingsMenuItem_scope_enum" DEFAULT 'WORKSPACE'::core."settingsMenuItem_scope_enum" NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    slug character varying
);


--
-- Name: signingKey; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."signingKey" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "publicKey" character varying NOT NULL,
    "privateKey" character varying,
    "isCurrent" boolean DEFAULT false NOT NULL,
    "revokedAt" timestamp with time zone,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT "CHK_signingKey_privateKey_encrypted" CHECK ((("privateKey" IS NULL) OR (("privateKey")::text ~~ 'enc:v2:%'::text)))
);


--
-- Name: skill; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core.skill (
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    label character varying NOT NULL,
    icon character varying,
    description text,
    content text NOT NULL,
    "isActive" boolean DEFAULT true NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "isSystem" boolean DEFAULT false NOT NULL,
    overrides jsonb
);


--
-- Name: timelineActivityType; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."timelineActivityType" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    label character varying NOT NULL,
    action character varying,
    icon character varying,
    "objectUniversalIdentifier" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "frontComponentUniversalIdentifier" uuid,
    "targetRelationFieldUniversalIdentifier" uuid,
    "triggerFieldUniversalIdentifiers" uuid[],
    "replacesTimelineActivityTypeUniversalIdentifier" uuid,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    "happensAtFieldUniversalIdentifier" uuid
);


--
-- Name: twoFactorAuthenticationMethod; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."twoFactorAuthenticationMethod" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "userWorkspaceId" uuid NOT NULL,
    secret text NOT NULL,
    status core."twoFactorAuthenticationMethod_status_enum" NOT NULL,
    strategy core."twoFactorAuthenticationMethod_strategy_enum" NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "workspaceId" uuid NOT NULL,
    CONSTRAINT "CHK_twoFactorAuthenticationMethod_secret_encrypted" CHECK ((secret ~~ 'enc:v2:%'::text))
);


--
-- Name: unsubscribeTopic; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."unsubscribeTopic" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    name character varying,
    description character varying,
    visibility core."unsubscribeTopic_visibility_enum" DEFAULT 'PRIVATE'::core."unsubscribeTopic_visibility_enum" NOT NULL,
    "workspaceId" uuid NOT NULL
);


--
-- Name: upgradeMigration; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."upgradeMigration" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    status character varying NOT NULL,
    attempt integer DEFAULT 1 NOT NULL,
    "executedByVersion" character varying NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "workspaceId" uuid,
    "errorMessage" text,
    "isInitial" boolean DEFAULT false NOT NULL
);


--
-- Name: usageLimit; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."usageLimit" (
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "resourceType" character varying NOT NULL,
    "operationType" character varying NOT NULL,
    "spenderType" character varying NOT NULL,
    "spenderId" character varying DEFAULT ''::character varying NOT NULL,
    "limitKind" character varying NOT NULL,
    "periodCount" integer NOT NULL,
    "limitValue" bigint NOT NULL,
    "burstValue" bigint,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "periodUnit" character varying NOT NULL,
    "isInstanceOverride" boolean DEFAULT false NOT NULL,
    unit character varying NOT NULL
);


--
-- Name: user; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."user" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "firstName" character varying DEFAULT ''::character varying NOT NULL,
    "lastName" character varying DEFAULT ''::character varying NOT NULL,
    email public.citext NOT NULL,
    "isEmailVerified" boolean DEFAULT false NOT NULL,
    disabled boolean DEFAULT false NOT NULL,
    "passwordHash" character varying,
    "canImpersonate" boolean DEFAULT false NOT NULL,
    "canAccessFullAdminPanel" boolean DEFAULT false NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    locale character varying DEFAULT 'en'::character varying NOT NULL
);


--
-- Name: userApplicationVariableValue; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."userApplicationVariableValue" (
    "workspaceId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "applicationVariableId" uuid NOT NULL,
    scope core."applicationVariable_scope_enum" DEFAULT 'USER'::core."applicationVariable_scope_enum" NOT NULL,
    "userWorkspaceId" uuid NOT NULL,
    value text NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT "CHK_userApplicationVariableValue_scope_user" CHECK ((scope = 'USER'::core."applicationVariable_scope_enum")),
    CONSTRAINT "CHK_userApplicationVariableValue_value_encrypted" CHECK ((value ~~ 'enc:v2:%'::text))
);


--
-- Name: userSession; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."userSession" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "tokenHash" text NOT NULL,
    "userId" uuid NOT NULL,
    "workspaceId" uuid,
    "userWorkspaceId" uuid,
    "authProvider" text NOT NULL,
    "isImpersonating" boolean DEFAULT false NOT NULL,
    "impersonatorUserWorkspaceId" uuid,
    "impersonatedUserWorkspaceId" uuid,
    "userAgent" text,
    "ipAddress" text,
    "expiresAt" timestamp with time zone NOT NULL,
    "lastActiveAt" timestamp with time zone NOT NULL,
    "revokedAt" timestamp with time zone,
    "revokedReason" text,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    country text,
    region text,
    city text
);


--
-- Name: userWorkspace; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."userWorkspace" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "userId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    "defaultAvatarUrl" character varying,
    locale character varying DEFAULT 'en'::character varying NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "inboxNotificationLevel" character varying DEFAULT 'ALL_FOLLOWED'::character varying NOT NULL,
    "isInboxNotificationPreviewEnabled" boolean DEFAULT true NOT NULL
);


--
-- Name: validationRule; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."validationRule" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    "errorFieldMetadataId" uuid,
    name character varying NOT NULL,
    description text,
    icon character varying,
    expression text NOT NULL,
    bindings jsonb DEFAULT '{}'::jsonb NOT NULL,
    message text NOT NULL,
    "isActive" boolean DEFAULT true NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: view; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core.view (
    "universalIdentifier" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name text DEFAULT ''::text NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    type core.view_type_enum DEFAULT 'TABLE'::core.view_type_enum NOT NULL,
    key core.view_key_enum,
    icon text NOT NULL,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "isCompact" boolean DEFAULT false NOT NULL,
    "openRecordIn" core.view_openrecordin_enum DEFAULT 'SIDE_PANEL'::core.view_openrecordin_enum NOT NULL,
    "kanbanAggregateOperation" core.view_kanbanaggregateoperation_enum,
    "kanbanAggregateOperationFieldMetadataId" uuid,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "anyFieldFilterValue" text,
    "calendarLayout" core.view_calendarlayout_enum,
    "calendarFieldMetadataId" uuid,
    "applicationId" uuid NOT NULL,
    visibility core.view_visibility_enum DEFAULT 'WORKSPACE'::core.view_visibility_enum NOT NULL,
    "createdByUserWorkspaceId" uuid,
    "mainGroupByFieldMetadataId" uuid,
    "shouldHideEmptyGroups" boolean DEFAULT false NOT NULL,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL,
    "kanbanColumnWidth" integer,
    "calendarEndFieldMetadataId" uuid,
    "groupLoadLimit" integer DEFAULT 8 NOT NULL,
    "shouldDisplayGroupsAsTabs" boolean DEFAULT false NOT NULL,
    CONSTRAINT "CHK_VIEW_CALENDAR_INTEGRITY" CHECK (((type <> ALL (ARRAY['CALENDAR'::core.view_type_enum, 'CALENDAR_WIDGET'::core.view_type_enum])) OR (("calendarLayout" IS NOT NULL) AND ("calendarFieldMetadataId" IS NOT NULL))))
);


--
-- Name: viewField; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."viewField" (
    "universalIdentifier" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "fieldMetadataId" uuid NOT NULL,
    "isVisible" boolean DEFAULT true NOT NULL,
    size integer DEFAULT 0 NOT NULL,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "aggregateOperation" core."viewField_aggregateoperation_enum",
    "viewId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "applicationId" uuid NOT NULL,
    "viewFieldGroupId" uuid,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL,
    "isCanonical" boolean DEFAULT true NOT NULL,
    "relationTargetFieldMetadataId" uuid
);


--
-- Name: viewFieldGroup; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."viewFieldGroup" (
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name text NOT NULL,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "isVisible" boolean DEFAULT true NOT NULL,
    "viewId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    overrides jsonb,
    "isActive" boolean DEFAULT true NOT NULL,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL
);


--
-- Name: viewFilter; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."viewFilter" (
    "universalIdentifier" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "fieldMetadataId" uuid NOT NULL,
    operand core."viewFilter_operand_enum" DEFAULT 'CONTAINS'::core."viewFilter_operand_enum" NOT NULL,
    value jsonb NOT NULL,
    "viewFilterGroupId" uuid,
    "positionInViewFilterGroup" double precision,
    "subFieldName" text,
    "viewId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "applicationId" uuid NOT NULL,
    "relationTargetFieldMetadataId" uuid
);


--
-- Name: viewFilterGroup; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."viewFilterGroup" (
    "universalIdentifier" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "parentViewFilterGroupId" uuid,
    "logicalOperator" core."viewFilterGroup_logicaloperator_enum" DEFAULT 'AND'::core."viewFilterGroup_logicaloperator_enum" NOT NULL,
    "positionInViewFilterGroup" double precision,
    "viewId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "applicationId" uuid NOT NULL
);


--
-- Name: viewGroup; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."viewGroup" (
    "universalIdentifier" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "isVisible" boolean DEFAULT true NOT NULL,
    "fieldValue" text NOT NULL,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "viewId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "applicationId" uuid NOT NULL
);


--
-- Name: viewSort; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."viewSort" (
    "universalIdentifier" uuid NOT NULL,
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "fieldMetadataId" uuid NOT NULL,
    direction core."viewSort_direction_enum" DEFAULT 'ASC'::core."viewSort_direction_enum" NOT NULL,
    "viewId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "applicationId" uuid NOT NULL,
    "subFieldName" character varying,
    "relationTargetFieldMetadataId" uuid
);


--
-- Name: webPushSubscription; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."webPushSubscription" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "workspaceId" uuid NOT NULL,
    endpoint character varying NOT NULL,
    "browserPublicKey" character varying NOT NULL,
    "browserAuthSecret" character varying NOT NULL,
    "vapidPublicKey" character varying NOT NULL,
    "userSessionId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL
);


--
-- Name: webhook; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core.webhook (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "targetUrl" character varying NOT NULL,
    operations text[] DEFAULT '{*.*}'::text[] NOT NULL,
    description character varying,
    secret character varying NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL
);


--
-- Name: workflow; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core.workflow (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name text,
    "lastPublishedVersionId" uuid,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "workspaceWorkflowId" uuid,
    "lastPublishedCoreWorkflowVersionId" uuid,
    visibility character varying DEFAULT 'WORKSPACE'::character varying NOT NULL,
    "createdByUserWorkspaceId" uuid,
    "versionDefinitionHash" text,
    "isSystem" boolean DEFAULT false NOT NULL
);


--
-- Name: workflowVersion; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."workflowVersion" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    triggers jsonb,
    steps jsonb,
    status core."workflowVersion_status_enum" DEFAULT 'DRAFT'::core."workflowVersion_status_enum" NOT NULL,
    "workflowId" uuid,
    "workspaceId" uuid NOT NULL,
    "universalIdentifier" uuid NOT NULL,
    "applicationId" uuid NOT NULL,
    "coreWorkflowId" uuid NOT NULL,
    "workspaceWorkflowVersionId" uuid,
    "isSystemSideEffect" boolean DEFAULT false NOT NULL
);


--
-- Name: workspace; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core.workspace (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "displayName" character varying,
    logo character varying,
    "inviteHash" character varying,
    "deletedAt" timestamp with time zone,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "allowImpersonation" boolean DEFAULT true NOT NULL,
    "isPublicInviteLinkEnabled" boolean DEFAULT true NOT NULL,
    "activationStatus" core."workspace_activationStatus_enum" DEFAULT 'INACTIVE'::core."workspace_activationStatus_enum" NOT NULL,
    "metadataVersion" integer DEFAULT 1 NOT NULL,
    "databaseSchema" character varying,
    subdomain character varying NOT NULL,
    "customDomain" character varying,
    "isGoogleAuthEnabled" boolean DEFAULT true NOT NULL,
    "isTwoFactorAuthenticationEnforced" boolean DEFAULT false NOT NULL,
    "isPasswordAuthEnabled" boolean DEFAULT true NOT NULL,
    "isMicrosoftAuthEnabled" boolean DEFAULT true NOT NULL,
    "isCustomDomainEnabled" boolean DEFAULT false NOT NULL,
    "defaultRoleId" uuid,
    "trashRetentionDays" integer DEFAULT 14 NOT NULL,
    "isGoogleAuthBypassEnabled" boolean DEFAULT false NOT NULL,
    "isPasswordAuthBypassEnabled" boolean DEFAULT false NOT NULL,
    "isMicrosoftAuthBypassEnabled" boolean DEFAULT false NOT NULL,
    "workspaceCustomApplicationId" uuid NOT NULL,
    "editableProfileFields" character varying[] DEFAULT '{email,profilePicture,firstName,lastName}'::character varying[],
    "eventLogRetentionDays" integer DEFAULT 3 NOT NULL,
    "suspendedAt" timestamp with time zone,
    "aiAdditionalInstructions" text,
    "logoFileId" uuid,
    "isInternalMessagesImportEnabled" boolean DEFAULT false NOT NULL,
    "workspaceDiscoverability" core.workspace_discoverability_enum DEFAULT 'PUBLIC'::core.workspace_discoverability_enum NOT NULL,
    "aiChatModelTier" character varying DEFAULT 'fast'::character varying NOT NULL,
    "aiAgentModelTier" character varying DEFAULT 'fast'::character varying NOT NULL,
    "isAutoModelSelectionEnabled" boolean DEFAULT true NOT NULL,
    "aiModelIdByTier" jsonb DEFAULT '{}'::jsonb NOT NULL,
    "aiEvaluationModelId" text,
    "allowedEmbeddingOrigins" character varying[] DEFAULT '{}'::character varying[] NOT NULL,
    "isCampaignClickTrackingEnabled" boolean DEFAULT false NOT NULL,
    "isCampaignOpenTrackingEnabled" boolean DEFAULT false NOT NULL,
    "aiChatDirectMessageAssistantMode" character varying DEFAULT 'MENTION_ONLY'::character varying NOT NULL,
    "aiChatInboxAssistantMode" character varying DEFAULT 'AUTOMATIC'::character varying NOT NULL,
    "iframeMediaPermissions" jsonb DEFAULT '[]'::jsonb NOT NULL,
    "aiChatAssistantProfile" jsonb DEFAULT '{"avatarFace": "default", "avatarColor": "blue", "avatarShape": "circle", "displayName": "Ada", "isAvatarAnimated": true}'::jsonb NOT NULL,
    "aiDataPolicy" jsonb DEFAULT '{"isZeroDataRetentionRequired": false, "requiredDataResidencyRegion": null, "isProductImprovementDataSharingEnabled": true}'::jsonb NOT NULL,
    "isInternalImportEnabled" boolean DEFAULT false NOT NULL,
    CONSTRAINT onboarded_workspace_requires_default_role CHECK ((("activationStatus" = ANY (ARRAY['PENDING_CREATION'::core."workspace_activationStatus_enum", 'ONGOING_CREATION'::core."workspace_activationStatus_enum"])) OR ("defaultRoleId" IS NOT NULL))),
    CONSTRAINT workspace_requires_database_schema CHECK ((("activationStatus" = ANY (ARRAY['PENDING_CREATION'::core."workspace_activationStatus_enum", 'ONGOING_CREATION'::core."workspace_activationStatus_enum"])) OR (("databaseSchema" IS NOT NULL) AND (("databaseSchema")::text <> ''::text))))
);


--
-- Name: workspaceSSOIdentityProvider; Type: TABLE; Schema: core; Owner: -
--

CREATE TABLE core."workspaceSSOIdentityProvider" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    name character varying NOT NULL,
    status core."workspaceSSOIdentityProvider_status_enum" DEFAULT 'Active'::core."workspaceSSOIdentityProvider_status_enum" NOT NULL,
    "workspaceId" uuid NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    type core."workspaceSSOIdentityProvider_type_enum" DEFAULT 'OIDC'::core."workspaceSSOIdentityProvider_type_enum" NOT NULL,
    issuer character varying NOT NULL,
    "clientID" character varying,
    "clientSecret" character varying,
    "ssoURL" character varying,
    certificate character varying,
    fingerprint character varying
);


--
-- Name: _typeorm_migrations id; Type: DEFAULT; Schema: core; Owner: -
--

ALTER TABLE ONLY core._typeorm_migrations ALTER COLUMN id SET DEFAULT nextval('core._typeorm_migrations_id_seq'::regclass);


--
-- Name: file CHK_FILE_PENDING_MIME_OCTET_STREAM; Type: CHECK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE core.file
    ADD CONSTRAINT "CHK_FILE_PENDING_MIME_OCTET_STREAM" CHECK (((status <> 'PENDING'::core.file_status_enum) OR (("mimeType")::text = 'application/octet-stream'::text))) NOT VALID;


--
-- Name: agentChatChannelRole IDX_AGENT_CHAT_CHANNEL_ROLE_CHANNEL_ID_ROLE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."agentChatChannelRole"
    ADD CONSTRAINT "IDX_AGENT_CHAT_CHANNEL_ROLE_CHANNEL_ID_ROLE_ID_UNIQUE" UNIQUE ("channelId", "roleId");


--
-- Name: applicationVariable IDX_APPLICATION_VARIABLE_ID_SCOPE_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationVariable"
    ADD CONSTRAINT "IDX_APPLICATION_VARIABLE_ID_SCOPE_UNIQUE" UNIQUE (id, scope);


--
-- Name: approvedAccessDomain IDX_APPROVED_ACCESS_DOMAIN_DOMAIN_WORKSPACE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."approvedAccessDomain"
    ADD CONSTRAINT "IDX_APPROVED_ACCESS_DOMAIN_DOMAIN_WORKSPACE_ID_UNIQUE" UNIQUE (domain, "workspaceId");


--
-- Name: applicationRegistrationVariable IDX_APP_REG_VAR_KEY_APP_REGISTRATION_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistrationVariable"
    ADD CONSTRAINT "IDX_APP_REG_VAR_KEY_APP_REGISTRATION_ID_UNIQUE" UNIQUE (key, "applicationRegistrationId");


--
-- Name: applicationRegistrationVersion IDX_APP_REG_VERSION_APP_REGISTRATION_ID_VERSION_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistrationVersion"
    ADD CONSTRAINT "IDX_APP_REG_VERSION_APP_REGISTRATION_ID_VERSION_UNIQUE" UNIQUE ("applicationRegistrationId", version);


--
-- Name: connectionProvider IDX_CONNECTION_PROVIDER_NAME_APPLICATION_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectionProvider"
    ADD CONSTRAINT "IDX_CONNECTION_PROVIDER_NAME_APPLICATION_UNIQUE" UNIQUE (name, "applicationId");


--
-- Name: emailingDomain IDX_EMAILING_DOMAIN_DOMAIN_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."emailingDomain"
    ADD CONSTRAINT "IDX_EMAILING_DOMAIN_DOMAIN_UNIQUE" UNIQUE (domain);


--
-- Name: featureFlag IDX_FEATURE_FLAG_KEY_WORKSPACE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."featureFlag"
    ADD CONSTRAINT "IDX_FEATURE_FLAG_KEY_WORKSPACE_ID_UNIQUE" UNIQUE (key, "workspaceId");


--
-- Name: fieldMetadata IDX_FIELD_METADATA_NAME_OBJECT_METADATA_ID_WORKSPACE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadata"
    ADD CONSTRAINT "IDX_FIELD_METADATA_NAME_OBJECT_METADATA_ID_WORKSPACE_ID_UNIQUE" UNIQUE (name, "objectMetadataId", "workspaceId");


--
-- Name: fieldMetadataOption IDX_FIELD_METADATA_OPTION_FIELD_METADATA_ID_VALUE_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadataOption"
    ADD CONSTRAINT "IDX_FIELD_METADATA_OPTION_FIELD_METADATA_ID_VALUE_UNIQUE" UNIQUE ("fieldMetadataId", value) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: file IDX_FILE_APPLICATION_ID_WORKSPACE_ID_PATH_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.file
    ADD CONSTRAINT "IDX_FILE_APPLICATION_ID_WORKSPACE_ID_PATH_UNIQUE" UNIQUE ("applicationId", "workspaceId", path);


--
-- Name: file IDX_FILE_APPLICATION_REGISTRATION_ID_PATH_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.file
    ADD CONSTRAINT "IDX_FILE_APPLICATION_REGISTRATION_ID_PATH_UNIQUE" UNIQUE ("applicationRegistrationId", path);


--
-- Name: indexMetadata IDX_INDEX_METADATA_NAME_WORKSPACE_ID_OBJECT_METADATA_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."indexMetadata"
    ADD CONSTRAINT "IDX_INDEX_METADATA_NAME_WORKSPACE_ID_OBJECT_METADATA_ID_UNIQUE" UNIQUE (name, "workspaceId", "objectMetadataId");


--
-- Name: keyValuePair IDX_KEY_VALUE_PAIR_KEY_USER_ID_WORKSPACE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."keyValuePair"
    ADD CONSTRAINT "IDX_KEY_VALUE_PAIR_KEY_USER_ID_WORKSPACE_ID_UNIQUE" UNIQUE (key, "userId", "workspaceId");


--
-- Name: messagingProvider IDX_MESSAGING_PROVIDER_NAME_APPLICATION_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messagingProvider"
    ADD CONSTRAINT "IDX_MESSAGING_PROVIDER_NAME_APPLICATION_UNIQUE" UNIQUE (name, "applicationId");


--
-- Name: objectMetadata IDX_OBJECT_METADATA_NAME_PLURAL_WORKSPACE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectMetadata"
    ADD CONSTRAINT "IDX_OBJECT_METADATA_NAME_PLURAL_WORKSPACE_ID_UNIQUE" UNIQUE ("namePlural", "workspaceId");


--
-- Name: objectMetadata IDX_OBJECT_METADATA_NAME_SINGULAR_WORKSPACE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectMetadata"
    ADD CONSTRAINT "IDX_OBJECT_METADATA_NAME_SINGULAR_WORKSPACE_ID_UNIQUE" UNIQUE ("nameSingular", "workspaceId");


--
-- Name: objectPermission IDX_OBJECT_PERMISSION_OBJECT_METADATA_ID_ROLE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectPermission"
    ADD CONSTRAINT "IDX_OBJECT_PERMISSION_OBJECT_METADATA_ID_ROLE_ID_UNIQUE" UNIQUE ("objectMetadataId", "roleId");


--
-- Name: permissionFlag IDX_PERMISSION_FLAG_KEY_WORKSPACE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."permissionFlag"
    ADD CONSTRAINT "IDX_PERMISSION_FLAG_KEY_WORKSPACE_ID_UNIQUE" UNIQUE (key, "workspaceId");


--
-- Name: role IDX_ROLE_LABEL_WORKSPACE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.role
    ADD CONSTRAINT "IDX_ROLE_LABEL_WORKSPACE_ID_UNIQUE" UNIQUE (label, "workspaceId");


--
-- Name: rolePermissionFlag IDX_ROLE_PERMISSION_FLAG_PERMISSION_FLAG_ID_ROLE_ID_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rolePermissionFlag"
    ADD CONSTRAINT "IDX_ROLE_PERMISSION_FLAG_PERMISSION_FLAG_ID_ROLE_ID_UNIQUE" UNIQUE ("permissionFlagId", "roleId");


--
-- Name: roleTarget IDX_ROLE_TARGET_UNIQUE_AGENT; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "IDX_ROLE_TARGET_UNIQUE_AGENT" UNIQUE ("workspaceId", "agentId");


--
-- Name: roleTarget IDX_ROLE_TARGET_UNIQUE_API_KEY; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "IDX_ROLE_TARGET_UNIQUE_API_KEY" UNIQUE ("workspaceId", "apiKeyId");


--
-- Name: roleTarget IDX_ROLE_TARGET_UNIQUE_TARGET_APPLICATION; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "IDX_ROLE_TARGET_UNIQUE_TARGET_APPLICATION" UNIQUE ("workspaceId", "targetApplicationId");


--
-- Name: roleTarget IDX_ROLE_TARGET_UNIQUE_WEBHOOK; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "IDX_ROLE_TARGET_UNIQUE_WEBHOOK" UNIQUE ("workspaceId", "webhookId");


--
-- Name: searchFieldMetadata IDX_SEARCH_FIELD_METADATA_FIELD_OBJECT_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."searchFieldMetadata"
    ADD CONSTRAINT "IDX_SEARCH_FIELD_METADATA_FIELD_OBJECT_UNIQUE" UNIQUE ("fieldMetadataId", "objectMetadataId");


--
-- Name: timelineActivityType IDX_TIMELINE_ACTIVITY_TYPE_APPLICATION_WORKSPACE_NAME_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."timelineActivityType"
    ADD CONSTRAINT "IDX_TIMELINE_ACTIVITY_TYPE_APPLICATION_WORKSPACE_NAME_UNIQUE" UNIQUE ("applicationId", "workspaceId", name);


--
-- Name: userApplicationVariableValue IDX_USER_APPLICATION_VARIABLE_VALUE_VARIABLE_USER_UNIQUE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userApplicationVariableValue"
    ADD CONSTRAINT "IDX_USER_APPLICATION_VARIABLE_VALUE_VARIABLE_USER_UNIQUE" UNIQUE ("applicationVariableId", "userWorkspaceId");


--
-- Name: viewFieldGroup PK_006f1cb78ab9eeef56c3e305009; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFieldGroup"
    ADD CONSTRAINT "PK_006f1cb78ab9eeef56c3e305009" PRIMARY KEY (id);


--
-- Name: searchFieldMetadata PK_085190eb7531f4aeb8ccab3f42c; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."searchFieldMetadata"
    ADD CONSTRAINT "PK_085190eb7531f4aeb8ccab3f42c" PRIMARY KEY (id);


--
-- Name: viewFilter PK_09f9ffa2f66263b9eb301460137; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilter"
    ADD CONSTRAINT "PK_09f9ffa2f66263b9eb301460137" PRIMARY KEY (id);


--
-- Name: roleTarget PK_0fe0b3be0a4a966e76c00f44df9; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "PK_0fe0b3be0a4a966e76c00f44df9" PRIMARY KEY (id);


--
-- Name: agent PK_1000e989398c5d4ed585cf9a46f; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.agent
    ADD CONSTRAINT "PK_1000e989398c5d4ed585cf9a46f" PRIMARY KEY (id);


--
-- Name: agentChatChannelRole PK_121b99ba798cf59c70481400d47; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."agentChatChannelRole"
    ADD CONSTRAINT "PK_121b99ba798cf59c70481400d47" PRIMARY KEY (id);


--
-- Name: appToken PK_143bfe36c6284c6d3a52c94741f; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."appToken"
    ADD CONSTRAINT "PK_143bfe36c6284c6d3a52c94741f" PRIMARY KEY (id);


--
-- Name: viewFilterGroup PK_16f55359d609168b826405ed307; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilterGroup"
    ADD CONSTRAINT "PK_16f55359d609168b826405ed307" PRIMARY KEY (id);


--
-- Name: userWorkspace PK_222871f3641385e36e0b9f82aeb; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userWorkspace"
    ADD CONSTRAINT "PK_222871f3641385e36e0b9f82aeb" PRIMARY KEY (id);


--
-- Name: objectPermission PK_23a4033c1aa380d0d1431731add; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectPermission"
    ADD CONSTRAINT "PK_23a4033c1aa380d0d1431731add" PRIMARY KEY (id);


--
-- Name: validationRule PK_293a30050cd51815f8ed80780e4; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."validationRule"
    ADD CONSTRAINT "PK_293a30050cd51815f8ed80780e4" PRIMARY KEY (id);


--
-- Name: apiKey PK_2ae3a5e8e04fb402b2dc8d6ce4b; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."apiKey"
    ADD CONSTRAINT "PK_2ae3a5e8e04fb402b2dc8d6ce4b" PRIMARY KEY (id);


--
-- Name: pageLayoutWidget PK_2f997489b8b15cb26a0b9d4220b; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayoutWidget"
    ADD CONSTRAINT "PK_2f997489b8b15cb26a0b9d4220b" PRIMARY KEY (id);


--
-- Name: file PK_36b46d232307066b3a2c9ea3a1d; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.file
    ADD CONSTRAINT "PK_36b46d232307066b3a2c9ea3a1d" PRIMARY KEY (id);


--
-- Name: connectionProviderOAuthClient PK_378876ac9fa9e1d9177a7f2c790; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectionProviderOAuthClient"
    ADD CONSTRAINT "PK_378876ac9fa9e1d9177a7f2c790" PRIMARY KEY (id);


--
-- Name: messageChannel PK_438b9412475f39712ed065f77af; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messageChannel"
    ADD CONSTRAINT "PK_438b9412475f39712ed065f77af" PRIMARY KEY (id);


--
-- Name: mcpServer PK_467ff520c3aef4efa94e2b6afe6; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."mcpServer"
    ADD CONSTRAINT "PK_467ff520c3aef4efa94e2b6afe6" PRIMARY KEY (id);


--
-- Name: logicFunction PK_49bfacee064bee9d0d486483b60; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."logicFunction"
    ADD CONSTRAINT "PK_49bfacee064bee9d0d486483b60" PRIMARY KEY (id);


--
-- Name: pageLayout PK_5028ccb46ffa0c945d2f9246dfa; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayout"
    ADD CONSTRAINT "PK_5028ccb46ffa0c945d2f9246dfa" PRIMARY KEY (id);


--
-- Name: rowLevelPermissionPredicateGroup PK_5084d63eb632c38d70b974841f3; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicateGroup"
    ADD CONSTRAINT "PK_5084d63eb632c38d70b974841f3" PRIMARY KEY (id);


--
-- Name: relatedRecordPath PK_5105d98eee85b6a1e674109ba3d; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."relatedRecordPath"
    ADD CONSTRAINT "PK_5105d98eee85b6a1e674109ba3d" PRIMARY KEY (id);


--
-- Name: userApplicationVariableValue PK_51df8f8b24eb8c05e07a3542f92; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userApplicationVariableValue"
    ADD CONSTRAINT "PK_51df8f8b24eb8c05e07a3542f92" PRIMARY KEY (id);


--
-- Name: approvedAccessDomain PK_523281ce57c84e1a039f4538c19; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."approvedAccessDomain"
    ADD CONSTRAINT "PK_523281ce57c84e1a039f4538c19" PRIMARY KEY (id);


--
-- Name: webPushSubscription PK_56665eaf0faa83047c4c8300285; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."webPushSubscription"
    ADD CONSTRAINT "PK_56665eaf0faa83047c4c8300285" PRIMARY KEY (id);


--
-- Name: application PK_569e0c3e863ebdf5f2408ee1670; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "PK_569e0c3e863ebdf5f2408ee1670" PRIMARY KEY (id);


--
-- Name: indexFieldMetadata PK_5928f67e43eff7d95aa79fd96fd; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."indexFieldMetadata"
    ADD CONSTRAINT "PK_5928f67e43eff7d95aa79fd96fd" PRIMARY KEY (id);


--
-- Name: applicationVariable PK_62f7823eb5f1e416c9d60614dfb; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationVariable"
    ADD CONSTRAINT "PK_62f7823eb5f1e416c9d60614dfb" PRIMARY KEY (id);


--
-- Name: timelineActivityType PK_65c4ac97169ca5241f0f17b5a44; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."timelineActivityType"
    ADD CONSTRAINT "PK_65c4ac97169ca5241f0f17b5a44" PRIMARY KEY (id);


--
-- Name: dataSource PK_6d01ae6c0f47baf4f8e37342268; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."dataSource"
    ADD CONSTRAINT "PK_6d01ae6c0f47baf4f8e37342268" PRIMARY KEY (id);


--
-- Name: rolePermissionFlag PK_76591adc8035c2e7b0cd6115136; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rolePermissionFlag"
    ADD CONSTRAINT "PK_76591adc8035c2e7b0cd6115136" PRIMARY KEY (id);


--
-- Name: calendarChannel PK_796d701c0c35518517d0f3e0e0b; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."calendarChannel"
    ADD CONSTRAINT "PK_796d701c0c35518517d0f3e0e0b" PRIMARY KEY (id);


--
-- Name: settingsMenuItem PK_7e404463b7409f267826d27cecd; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."settingsMenuItem"
    ADD CONSTRAINT "PK_7e404463b7409f267826d27cecd" PRIMARY KEY (id);


--
-- Name: objectMetadata PK_81fb7f4f4244211cfbd188af1e8; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectMetadata"
    ADD CONSTRAINT "PK_81fb7f4f4244211cfbd188af1e8" PRIMARY KEY (id);


--
-- Name: frontComponent PK_843479d93ef40e58dc4587339aa; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."frontComponent"
    ADD CONSTRAINT "PK_843479d93ef40e58dc4587339aa" PRIMARY KEY (id);


--
-- Name: messageFolder PK_85cb5a339d9f7f1106dde9e4db8; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messageFolder"
    ADD CONSTRAINT "PK_85cb5a339d9f7f1106dde9e4db8" PRIMARY KEY (id);


--
-- Name: view PK_86cfb9e426c77d60b900fe2b543; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.view
    ADD CONSTRAINT "PK_86cfb9e426c77d60b900fe2b543" PRIMARY KEY (id);


--
-- Name: featureFlag PK_894efa1b1822de801f3b9e04069; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."featureFlag"
    ADD CONSTRAINT "PK_894efa1b1822de801f3b9e04069" PRIMARY KEY (id);


--
-- Name: connectedAccount PK_8e7a0a0bbc2e06ac2acf89b7f3a; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectedAccount"
    ADD CONSTRAINT "PK_8e7a0a0bbc2e06ac2acf89b7f3a" PRIMARY KEY (id);


--
-- Name: rowLevelPermissionRule PK_97afab627d6cf66841cbf92ccd2; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionRule"
    ADD CONSTRAINT "PK_97afab627d6cf66841cbf92ccd2" PRIMARY KEY (id);


--
-- Name: permissionFlag PK_a02789db60620a1e9f90147b50f; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."permissionFlag"
    ADD CONSTRAINT "PK_a02789db60620a1e9f90147b50f" PRIMARY KEY (id);


--
-- Name: logicFunctionLayer PK_a1077708d1b19463ab2eda7c246; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."logicFunctionLayer"
    ADD CONSTRAINT "PK_a1077708d1b19463ab2eda7c246" PRIMARY KEY (id);


--
-- Name: upgradeMigration PK_a43ea44de07f51fdc55b88af2ad; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."upgradeMigration"
    ADD CONSTRAINT "PK_a43ea44de07f51fdc55b88af2ad" PRIMARY KEY (id);


--
-- Name: workspaceSSOIdentityProvider PK_a4e3928eb641e7cd612042b628b; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."workspaceSSOIdentityProvider"
    ADD CONSTRAINT "PK_a4e3928eb641e7cd612042b628b" PRIMARY KEY (id);


--
-- Name: skill PK_a5167c44f4d4e61423f7f5e43bf; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.skill
    ADD CONSTRAINT "PK_a5167c44f4d4e61423f7f5e43bf" PRIMARY KEY (id);


--
-- Name: _typeorm_migrations PK_a6ff2a8e8bb563f3d15635efd01; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core._typeorm_migrations
    ADD CONSTRAINT "PK_a6ff2a8e8bb563f3d15635efd01" PRIMARY KEY (id);


--
-- Name: usageLimit PK_aa789af8757d2ef28a0416df290; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."usageLimit"
    ADD CONSTRAINT "PK_aa789af8757d2ef28a0416df290" PRIMARY KEY (id);


--
-- Name: applicationAuthorization PK_applicationAuthorization_id; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationAuthorization"
    ADD CONSTRAINT "PK_applicationAuthorization_id" PRIMARY KEY (id);


--
-- Name: applicationTranslation PK_applicationTranslation_id; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationTranslation"
    ADD CONSTRAINT "PK_applicationTranslation_id" PRIMARY KEY (id);


--
-- Name: applicationRegistration PK_application_registration; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistration"
    ADD CONSTRAINT "PK_application_registration" PRIMARY KEY (id);


--
-- Name: applicationRegistrationVariable PK_application_registration_variable; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistrationVariable"
    ADD CONSTRAINT "PK_application_registration_variable" PRIMARY KEY (id);


--
-- Name: role PK_b36bcfe02fc8de3c57a8b2391c2; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.role
    ADD CONSTRAINT "PK_b36bcfe02fc8de3c57a8b2391c2" PRIMARY KEY (id);


--
-- Name: viewField PK_ba2a5aa5f0bd7ac82788fae921e; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewField"
    ADD CONSTRAINT "PK_ba2a5aa5f0bd7ac82788fae921e" PRIMARY KEY (id);


--
-- Name: applicationRegistrationVersion PK_bfa9041162e158e878354984852; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistrationVersion"
    ADD CONSTRAINT "PK_bfa9041162e158e878354984852" PRIMARY KEY (id);


--
-- Name: deferredWorkspaceOperation PK_c42e6d7a0bef682c6183ed7324c; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."deferredWorkspaceOperation"
    ADD CONSTRAINT "PK_c42e6d7a0bef682c6183ed7324c" PRIMARY KEY (id);


--
-- Name: twoFactorAuthenticationMethod PK_c455f6a499e7110fc95e4bea540; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."twoFactorAuthenticationMethod"
    ADD CONSTRAINT "PK_c455f6a499e7110fc95e4bea540" PRIMARY KEY (id);


--
-- Name: keyValuePair PK_c5a1ca828435d3eaf8f9361ed4b; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."keyValuePair"
    ADD CONSTRAINT "PK_c5a1ca828435d3eaf8f9361ed4b" PRIMARY KEY (id);


--
-- Name: fieldMetadataOption PK_c9060d4696fdbd5f86ecebd7fb1; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadataOption"
    ADD CONSTRAINT "PK_c9060d4696fdbd5f86ecebd7fb1" PRIMARY KEY (id);


--
-- Name: workspace PK_ca86b6f9b3be5fe26d307d09b49; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workspace
    ADD CONSTRAINT "PK_ca86b6f9b3be5fe26d307d09b49" PRIMARY KEY (id);


--
-- Name: user PK_cace4a159ff9f2512dd42373760; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."user"
    ADD CONSTRAINT "PK_cace4a159ff9f2512dd42373760" PRIMARY KEY (id);


--
-- Name: pendingWakeUp PK_cbc9e34921b29ba23534bc3a2e5; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pendingWakeUp"
    ADD CONSTRAINT "PK_cbc9e34921b29ba23534bc3a2e5" PRIMARY KEY (id);


--
-- Name: messagingProvider PK_cf0b52a057bbef9c4975ab083e0; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messagingProvider"
    ADD CONSTRAINT "PK_cf0b52a057bbef9c4975ab083e0" PRIMARY KEY (id);


--
-- Name: connectionProvider PK_connectionProvider_id; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectionProvider"
    ADD CONSTRAINT "PK_connectionProvider_id" PRIMARY KEY (id);


--
-- Name: fieldMetadata PK_d046b1c7cea325ebc4cdc25e7a9; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadata"
    ADD CONSTRAINT "PK_d046b1c7cea325ebc4cdc25e7a9" PRIMARY KEY (id);


--
-- Name: viewGroup PK_d2aa8cad01e9d5e99c23f9ccec3; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewGroup"
    ADD CONSTRAINT "PK_d2aa8cad01e9d5e99c23f9ccec3" PRIMARY KEY (id);


--
-- Name: agentChatChannel PK_d74b9453b964418b53ca23f4d32; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."agentChatChannel"
    ADD CONSTRAINT "PK_d74b9453b964418b53ca23f4d32" PRIMARY KEY (id);


--
-- Name: fieldPermission PK_d7bb911e4f9b1b5e3bfcfdd1c4b; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldPermission"
    ADD CONSTRAINT "PK_d7bb911e4f9b1b5e3bfcfdd1c4b" PRIMARY KEY (id);


--
-- Name: navigationMenuItem PK_d8689756f55769faea7dc0ae968; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "PK_d8689756f55769faea7dc0ae968" PRIMARY KEY (id);


--
-- Name: emailingDomain PK_dca7032537b5d307f8cc6d74f1d; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."emailingDomain"
    ADD CONSTRAINT "PK_dca7032537b5d307f8cc6d74f1d" PRIMARY KEY (id);


--
-- Name: dpaAgreement PK_dpaAgreement_id; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."dpaAgreement"
    ADD CONSTRAINT "PK_dpaAgreement_id" PRIMARY KEY (id);


--
-- Name: webhook PK_e6765510c2d078db49632b59020; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.webhook
    ADD CONSTRAINT "PK_e6765510c2d078db49632b59020" PRIMARY KEY (id);


--
-- Name: rowLevelPermissionPredicate PK_e7ac2b75856fc7f300b5feb0e39; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "PK_e7ac2b75856fc7f300b5feb0e39" PRIMARY KEY (id);


--
-- Name: workflow PK_eb5e4cc1a9ef2e94805b676751b; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workflow
    ADD CONSTRAINT "PK_eb5e4cc1a9ef2e94805b676751b" PRIMARY KEY (id);


--
-- Name: viewSort PK_eceb74d297f926313af6463d496; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewSort"
    ADD CONSTRAINT "PK_eceb74d297f926313af6463d496" PRIMARY KEY (id);


--
-- Name: pageLayoutTab PK_f1327f6ea950cdc59fe17569c5c; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayoutTab"
    ADD CONSTRAINT "PK_f1327f6ea950cdc59fe17569c5c" PRIMARY KEY (id);


--
-- Name: indexMetadata PK_f73bb3c3678aee204e341f0ca4e; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."indexMetadata"
    ADD CONSTRAINT "PK_f73bb3c3678aee204e341f0ca4e" PRIMARY KEY (id);


--
-- Name: commandMenuItem PK_fd076dc869e721593133fe8a007; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."commandMenuItem"
    ADD CONSTRAINT "PK_fd076dc869e721593133fe8a007" PRIMARY KEY (id);


--
-- Name: publicDomain PK_ff55a0f1bc3b6e2c32feff734b1; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."publicDomain"
    ADD CONSTRAINT "PK_ff55a0f1bc3b6e2c32feff734b1" PRIMARY KEY (id);


--
-- Name: signingKey PK_signingKey_id; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."signingKey"
    ADD CONSTRAINT "PK_signingKey_id" PRIMARY KEY (id);


--
-- Name: unsubscribeTopic PK_unsubscribeTopic_id; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."unsubscribeTopic"
    ADD CONSTRAINT "PK_unsubscribeTopic_id" PRIMARY KEY (id);


--
-- Name: userSession PK_userSession_id; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userSession"
    ADD CONSTRAINT "PK_userSession_id" PRIMARY KEY (id);


--
-- Name: workflowVersion PK_workflowVersion_id; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."workflowVersion"
    ADD CONSTRAINT "PK_workflowVersion_id" PRIMARY KEY (id);


--
-- Name: applicationRegistration REL_36715821de396df9536fd4afc8; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistration"
    ADD CONSTRAINT "REL_36715821de396df9536fd4afc8" UNIQUE ("tarballFileId");


--
-- Name: fieldMetadata REL_47a6c57e1652b6475f8248cff7; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadata"
    ADD CONSTRAINT "REL_47a6c57e1652b6475f8248cff7" UNIQUE ("relationTargetFieldMetadataId");


--
-- Name: publicDomain UQ_1311e24fbd049c561c53a274f2a; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."publicDomain"
    ADD CONSTRAINT "UQ_1311e24fbd049c561c53a274f2a" UNIQUE (domain);


--
-- Name: workspace UQ_282123b2f32e927b6003311e33a; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workspace
    ADD CONSTRAINT "UQ_282123b2f32e927b6003311e33a" UNIQUE ("logoFileId");


--
-- Name: application UQ_28f20711184b3c3318a8e44d117; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "UQ_28f20711184b3c3318a8e44d117" UNIQUE ("yarnLockFileId");


--
-- Name: application UQ_3818380258798f9ffa9963b6dc4; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "UQ_3818380258798f9ffa9963b6dc4" UNIQUE ("packageJsonFileId");


--
-- Name: application UQ_3d6ee2b75b81933c1708918f647; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "UQ_3d6ee2b75b81933c1708918f647" UNIQUE ("logoFileId");


--
-- Name: applicationRegistration UQ_796819fb23559c233e6ebd49f34; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistration"
    ADD CONSTRAINT "UQ_796819fb23559c233e6ebd49f34" UNIQUE ("logoFileId");


--
-- Name: workspace UQ_900f0a3eb789159c26c8bcb39cd; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workspace
    ADD CONSTRAINT "UQ_900f0a3eb789159c26c8bcb39cd" UNIQUE ("customDomain");


--
-- Name: pendingWakeUp UQ_PENDING_WAKE_UP_OWNER; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pendingWakeUp"
    ADD CONSTRAINT "UQ_PENDING_WAKE_UP_OWNER" UNIQUE ("ownerType", "ownerId", "ownerKey");


--
-- Name: usageLimit UQ_USAGE_LIMIT_SCOPE; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."usageLimit"
    ADD CONSTRAINT "UQ_USAGE_LIMIT_SCOPE" UNIQUE ("workspaceId", "resourceType", "operationType", "spenderType", "spenderId", "limitKind", "periodCount", "periodUnit", unit);


--
-- Name: workspace UQ_cba6255a24deb1fff07dd7351b8; Type: CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workspace
    ADD CONSTRAINT "UQ_cba6255a24deb1fff07dd7351b8" UNIQUE (subdomain);


--
-- Name: IDX_0082568653b80c15903c5a2ba9; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_0082568653b80c15903c5a2ba9" ON core."roleTarget" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_054f9c52cf0e890640683d0a4a; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_054f9c52cf0e890640683d0a4a" ON core."relatedRecordPath" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_0cc4d03dbcc269e77ba4d297fb; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_0cc4d03dbcc269e77ba4d297fb" ON core.agent USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_0dedb90c717e179ef653c512b9; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_0dedb90c717e179ef653c512b9" ON core."fieldPermission" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_19cf74c0bcf8386bbf43e11e22; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_19cf74c0bcf8386bbf43e11e22" ON core."agentChatChannel" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_256fabec226411154baba649df; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_256fabec226411154baba649df" ON core."pageLayout" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_2909f5139c479e4632df03fd5e; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_2909f5139c479e4632df03fd5e" ON core."twoFactorAuthenticationMethod" USING btree ("userWorkspaceId", strategy);


--
-- Name: IDX_2a33a0e7e44c393ca7bb578dae; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_2a33a0e7e44c393ca7bb578dae" ON core."pageLayoutWidget" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_2cabff7ba187115afcdd1cf6ba; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_2cabff7ba187115afcdd1cf6ba" ON core."timelineActivityType" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_2f0fd3da807fb993701619d0ac; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_2f0fd3da807fb993701619d0ac" ON core."logicFunction" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_3763c4e8f942ff1e24040a13a9; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_3763c4e8f942ff1e24040a13a9" ON core."pageLayoutTab" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_38232fc0c6567ed029c2b1a12c; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_38232fc0c6567ed029c2b1a12c" ON core."viewSort" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_3a00d35710f4227ded320fd96d; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_3a00d35710f4227ded320fd96d" ON core."objectMetadata" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_3b7ff27925c0959777682c1adc; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_3b7ff27925c0959777682c1adc" ON core.role USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_44a4fc17a91603c38daabfd4d8; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_44a4fc17a91603c38daabfd4d8" ON core."connectionProvider" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_44ecebdf70cbed17f89527b36b; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_44ecebdf70cbed17f89527b36b" ON core."applicationVariable" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_4d8beaebdfcd5d82ebe6e8b58f; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_4d8beaebdfcd5d82ebe6e8b58f" ON core."navigationMenuItem" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_552aa6908966e980099b3e5ebf; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_552aa6908966e980099b3e5ebf" ON core.view USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_8943119a119b01408f422b8212; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_8943119a119b01408f422b8212" ON core."fieldMetadataOption" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_8ef15993c0e4bb37d1ceb9b87d; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_8ef15993c0e4bb37d1ceb9b87d" ON core.workflow USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_993aee642e113094f9e22b0728; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_993aee642e113094f9e22b0728" ON core."rowLevelPermissionRule" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_AGENT_CHAT_CHANNEL_NAME_WORKSPACE_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_AGENT_CHAT_CHANNEL_NAME_WORKSPACE_ID_UNIQUE" ON core."agentChatChannel" USING btree (name, "workspaceId");


--
-- Name: IDX_AGENT_CHAT_CHANNEL_ROLE_WORKSPACE_ID_ROLE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_AGENT_CHAT_CHANNEL_ROLE_WORKSPACE_ID_ROLE_ID" ON core."agentChatChannelRole" USING btree ("workspaceId", "roleId");


--
-- Name: IDX_AGENT_CHAT_CHANNEL_WORKSPACE_ID_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_AGENT_CHAT_CHANNEL_WORKSPACE_ID_APPLICATION_ID" ON core."agentChatChannel" USING btree ("workspaceId", "applicationId");


--
-- Name: IDX_AGENT_CREATED_BY_USER_WORKSPACE; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_AGENT_CREATED_BY_USER_WORKSPACE" ON core.agent USING btree ("createdByUserWorkspaceId");


--
-- Name: IDX_AGENT_ID_DELETED_AT; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_AGENT_ID_DELETED_AT" ON core.agent USING btree (id, "deletedAt");


--
-- Name: IDX_AGENT_NAME_WORKSPACE_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_AGENT_NAME_WORKSPACE_ID_UNIQUE" ON core.agent USING btree (name, "workspaceId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_API_KEY_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_API_KEY_WORKSPACE_ID" ON core."apiKey" USING btree ("workspaceId");


--
-- Name: IDX_APPLICATION_AUTHORIZATION_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APPLICATION_AUTHORIZATION_APPLICATION_ID" ON core."applicationAuthorization" USING btree ("applicationId");


--
-- Name: IDX_APPLICATION_AUTHORIZATION_USER_APPLICATION_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_APPLICATION_AUTHORIZATION_USER_APPLICATION_UNIQUE" ON core."applicationAuthorization" USING btree ("userId", "applicationId");


--
-- Name: IDX_APPLICATION_AUTHORIZATION_USER_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APPLICATION_AUTHORIZATION_USER_WORKSPACE_ID" ON core."applicationAuthorization" USING btree ("userWorkspaceId");


--
-- Name: IDX_APPLICATION_AUTHORIZATION_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APPLICATION_AUTHORIZATION_WORKSPACE_ID" ON core."applicationAuthorization" USING btree ("workspaceId");


--
-- Name: IDX_APPLICATION_REGISTRATION_CREATED_BY_USER_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APPLICATION_REGISTRATION_CREATED_BY_USER_ID" ON core."applicationRegistration" USING btree ("createdByUserId");


--
-- Name: IDX_APPLICATION_REGISTRATION_OAUTH_CLIENT_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_APPLICATION_REGISTRATION_OAUTH_CLIENT_ID_UNIQUE" ON core."applicationRegistration" USING btree ("oAuthClientId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_APPLICATION_REGISTRATION_UNIVERSAL_IDENTIFIER_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_APPLICATION_REGISTRATION_UNIVERSAL_IDENTIFIER_UNIQUE" ON core."applicationRegistration" USING btree ("universalIdentifier") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_APPLICATION_REGISTRATION_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APPLICATION_REGISTRATION_WORKSPACE_ID" ON core."applicationRegistration" USING btree ("workspaceId");


--
-- Name: IDX_APPLICATION_TRANSLATION_REGISTRATION_LOCALE_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_APPLICATION_TRANSLATION_REGISTRATION_LOCALE_UNIQUE" ON core."applicationTranslation" USING btree ("applicationRegistrationId", locale) WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_APPLICATION_TRANSLATION_STANDARD_LOCALE_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_APPLICATION_TRANSLATION_STANDARD_LOCALE_UNIQUE" ON core."applicationTranslation" USING btree (locale) WHERE (("deletedAt" IS NULL) AND ("applicationRegistrationId" IS NULL));


--
-- Name: IDX_APPLICATION_UNIVERSAL_IDENTIFIER_WORKSPACE_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_APPLICATION_UNIVERSAL_IDENTIFIER_WORKSPACE_ID_UNIQUE" ON core.application USING btree ("universalIdentifier", "workspaceId") WHERE (("deletedAt" IS NULL) AND ("universalIdentifier" IS NOT NULL));


--
-- Name: IDX_APPLICATION_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APPLICATION_WORKSPACE_ID" ON core.application USING btree ("workspaceId");


--
-- Name: IDX_APP_REG_VAR_APP_REGISTRATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APP_REG_VAR_APP_REGISTRATION_ID" ON core."applicationRegistrationVariable" USING btree ("applicationRegistrationId");


--
-- Name: IDX_APP_REG_VERSION_TARBALL_FILE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APP_REG_VERSION_TARBALL_FILE_ID" ON core."applicationRegistrationVersion" USING btree ("tarballFileId");


--
-- Name: IDX_APP_TOKEN_RECOVERY_CODE_PENDING_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_APP_TOKEN_RECOVERY_CODE_PENDING_UNIQUE" ON core."appToken" USING btree ("userId", "workspaceId") WHERE ((type = 'TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE'::text) AND ("deletedAt" IS NULL) AND ("revokedAt" IS NULL));


--
-- Name: IDX_APP_TOKEN_TYPE_VALUE_SSO_EXCHANGE_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_APP_TOKEN_TYPE_VALUE_SSO_EXCHANGE_UNIQUE" ON core."appToken" USING btree (type, value) WHERE ((type = 'SSO_EXCHANGE_TOKEN'::text) AND ("deletedAt" IS NULL) AND ("revokedAt" IS NULL));


--
-- Name: IDX_APP_TOKEN_USER_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APP_TOKEN_USER_ID" ON core."appToken" USING btree ("userId") WHERE ("userId" IS NOT NULL);


--
-- Name: IDX_APP_TOKEN_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_APP_TOKEN_WORKSPACE_ID" ON core."appToken" USING btree ("workspaceId") WHERE ("workspaceId" IS NOT NULL);


--
-- Name: IDX_CALENDAR_CHANNEL_WEBHOOK_SUBSCRIPTION_EXTERNAL_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_CALENDAR_CHANNEL_WEBHOOK_SUBSCRIPTION_EXTERNAL_ID" ON core."calendarChannel" USING btree ("webhookSubscriptionExternalId") WHERE ("webhookSubscriptionExternalId" IS NOT NULL);


--
-- Name: IDX_CALENDAR_CHANNEL_WORKSPACE_ID_SYNC_ENABLED_SYNC_STAGE; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_CALENDAR_CHANNEL_WORKSPACE_ID_SYNC_ENABLED_SYNC_STAGE" ON core."calendarChannel" USING btree ("workspaceId", "isSyncEnabled", "syncStage");


--
-- Name: IDX_COMMAND_MENU_ITEM_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_COMMAND_MENU_ITEM_APPLICATION_ID" ON core."commandMenuItem" USING btree ("applicationId");


--
-- Name: IDX_COMMAND_MENU_ITEM_AVAILABILITY_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_COMMAND_MENU_ITEM_AVAILABILITY_OBJECT_METADATA_ID" ON core."commandMenuItem" USING btree ("availabilityObjectMetadataId");


--
-- Name: IDX_COMMAND_MENU_ITEM_FRONT_COMPONENT_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_COMMAND_MENU_ITEM_FRONT_COMPONENT_ID_WORKSPACE_ID" ON core."commandMenuItem" USING btree ("frontComponentId", "workspaceId");


--
-- Name: IDX_COMMAND_MENU_ITEM_NAVIGATION_TARGET_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_COMMAND_MENU_ITEM_NAVIGATION_TARGET_OBJECT_METADATA_ID" ON core."commandMenuItem" USING btree ("navigationTargetObjectMetadataId");


--
-- Name: IDX_COMMAND_MENU_ITEM_PAGE_LAYOUT_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_COMMAND_MENU_ITEM_PAGE_LAYOUT_ID_WORKSPACE_ID" ON core."commandMenuItem" USING btree ("pageLayoutId", "workspaceId");


--
-- Name: IDX_COMMAND_MENU_ITEM_WORKFLOW_VERSION_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_COMMAND_MENU_ITEM_WORKFLOW_VERSION_ID_WORKSPACE_ID" ON core."commandMenuItem" USING btree ("workflowVersionId", "workspaceId");


--
-- Name: IDX_CONNECTED_ACCOUNT_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_CONNECTED_ACCOUNT_APPLICATION_ID" ON core."connectedAccount" USING btree ("applicationId");


--
-- Name: IDX_CONNECTED_ACCOUNT_CONNECTION_PROVIDER_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_CONNECTED_ACCOUNT_CONNECTION_PROVIDER_ID" ON core."connectedAccount" USING btree ("connectionProviderId");


--
-- Name: IDX_CONNECTED_ACCOUNT_HANDLE_PROVIDER; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_CONNECTED_ACCOUNT_HANDLE_PROVIDER" ON core."connectedAccount" USING btree (handle, provider);


--
-- Name: IDX_CONNECTION_PROVIDER_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_CONNECTION_PROVIDER_APPLICATION_ID" ON core."connectionProvider" USING btree ("applicationId");


--
-- Name: IDX_CONNECTION_PROVIDER_OAUTH_CLIENT_PROVIDER_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_CONNECTION_PROVIDER_OAUTH_CLIENT_PROVIDER_ID_UNIQUE" ON core."connectionProviderOAuthClient" USING btree ("connectionProviderId");


--
-- Name: IDX_DATA_SOURCE_WORKSPACE_ID_CREATED_AT; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_DATA_SOURCE_WORKSPACE_ID_CREATED_AT" ON core."dataSource" USING btree ("workspaceId", "createdAt");


--
-- Name: IDX_DEFERRED_WORKSPACE_OPERATION_WORKSPACE_ID_STATUS; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_DEFERRED_WORKSPACE_OPERATION_WORKSPACE_ID_STATUS" ON core."deferredWorkspaceOperation" USING btree ("workspaceId", status);


--
-- Name: IDX_DPA_AGREEMENT_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_DPA_AGREEMENT_WORKSPACE_ID" ON core."dpaAgreement" USING btree ("workspaceId");


--
-- Name: IDX_FIELD_METADATA_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FIELD_METADATA_APPLICATION_ID" ON core."fieldMetadata" USING btree ("applicationId");


--
-- Name: IDX_FIELD_METADATA_OBJECT_METADATA_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FIELD_METADATA_OBJECT_METADATA_ID_WORKSPACE_ID" ON core."fieldMetadata" USING btree ("objectMetadataId", "workspaceId");


--
-- Name: IDX_FIELD_METADATA_OPTION_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FIELD_METADATA_OPTION_APPLICATION_ID" ON core."fieldMetadataOption" USING btree ("applicationId");


--
-- Name: IDX_FIELD_METADATA_RELATION_TARGET_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FIELD_METADATA_RELATION_TARGET_FIELD_METADATA_ID" ON core."fieldMetadata" USING btree ("relationTargetFieldMetadataId");


--
-- Name: IDX_FIELD_METADATA_RELATION_TARGET_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FIELD_METADATA_RELATION_TARGET_OBJECT_METADATA_ID" ON core."fieldMetadata" USING btree ("relationTargetObjectMetadataId");


--
-- Name: IDX_FIELD_PERMISSION_OBJECT_LEVEL_FIELD_ROLE_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_FIELD_PERMISSION_OBJECT_LEVEL_FIELD_ROLE_UNIQUE" ON core."fieldPermission" USING btree ("fieldMetadataId", "roleId") WHERE ("rowLevelPermissionPredicateGroupId" IS NULL);


--
-- Name: IDX_FIELD_PERMISSION_RLS_RULE_FIELD_ROLE_GROUP_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_FIELD_PERMISSION_RLS_RULE_FIELD_ROLE_GROUP_UNIQUE" ON core."fieldPermission" USING btree ("fieldMetadataId", "roleId", "rowLevelPermissionPredicateGroupId") WHERE ("rowLevelPermissionPredicateGroupId" IS NOT NULL);


--
-- Name: IDX_FIELD_PERMISSION_RULE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FIELD_PERMISSION_RULE_ID" ON core."fieldPermission" USING btree ("rowLevelPermissionRuleId");


--
-- Name: IDX_FIELD_PERMISSION_WORKSPACE_ID_ROLE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FIELD_PERMISSION_WORKSPACE_ID_ROLE_ID" ON core."fieldPermission" USING btree ("workspaceId", "roleId");


--
-- Name: IDX_FILE_STATUS; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FILE_STATUS" ON core.file USING btree (status);


--
-- Name: IDX_FILE_TEMPORARY_EXPIRES_AT; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FILE_TEMPORARY_EXPIRES_AT" ON core.file USING btree (((settings ->> 'expiresAt'::text))) WHERE ((status = 'UPLOADED'::core.file_status_enum) AND ("workspaceId" IS NOT NULL) AND ("deletedAt" IS NULL) AND (settings @> '{"isTemporaryFile": true}'::jsonb));


--
-- Name: IDX_FILE_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_FILE_WORKSPACE_ID" ON core.file USING btree ("workspaceId");


--
-- Name: IDX_INDEX_FIELD_METADATA_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_INDEX_FIELD_METADATA_FIELD_METADATA_ID" ON core."indexFieldMetadata" USING btree ("fieldMetadataId");


--
-- Name: IDX_INDEX_FIELD_METADATA_INDEX_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_INDEX_FIELD_METADATA_INDEX_METADATA_ID" ON core."indexFieldMetadata" USING btree ("indexMetadataId");


--
-- Name: IDX_INDEX_METADATA_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_INDEX_METADATA_APPLICATION_ID" ON core."indexMetadata" USING btree ("applicationId");


--
-- Name: IDX_INDEX_METADATA_OBJECT_METADATA_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_INDEX_METADATA_OBJECT_METADATA_ID_WORKSPACE_ID" ON core."indexMetadata" USING btree ("objectMetadataId", "workspaceId");


--
-- Name: IDX_KEY_VALUE_PAIR_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_KEY_VALUE_PAIR_APPLICATION_ID" ON core."keyValuePair" USING btree ("applicationId");


--
-- Name: IDX_KEY_VALUE_PAIR_KEY_APPLICATION_ID_GLOBAL_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_KEY_VALUE_PAIR_KEY_APPLICATION_ID_GLOBAL_UNIQUE" ON core."keyValuePair" USING btree (key, "applicationId") WHERE (("applicationId" IS NOT NULL) AND ("workspaceId" IS NULL));


--
-- Name: IDX_KEY_VALUE_PAIR_KEY_APPLICATION_ID_WORKSPACE_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_KEY_VALUE_PAIR_KEY_APPLICATION_ID_WORKSPACE_UNIQUE" ON core."keyValuePair" USING btree (key, "applicationId") WHERE (("applicationId" IS NOT NULL) AND ("workspaceId" IS NOT NULL));


--
-- Name: IDX_KEY_VALUE_PAIR_KEY_NULL_USER_ID_NULL_WORKSPACE_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_KEY_VALUE_PAIR_KEY_NULL_USER_ID_NULL_WORKSPACE_ID_UNIQUE" ON core."keyValuePair" USING btree (key) WHERE (("userId" IS NULL) AND ("workspaceId" IS NULL) AND ("applicationId" IS NULL));


--
-- Name: IDX_KEY_VALUE_PAIR_KEY_USER_ID_NULL_WORKSPACE_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_KEY_VALUE_PAIR_KEY_USER_ID_NULL_WORKSPACE_ID_UNIQUE" ON core."keyValuePair" USING btree (key, "userId") WHERE ("workspaceId" IS NULL);


--
-- Name: IDX_KEY_VALUE_PAIR_KEY_WORKSPACE_ID_NULL_USER_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_KEY_VALUE_PAIR_KEY_WORKSPACE_ID_NULL_USER_ID_UNIQUE" ON core."keyValuePair" USING btree (key, "workspaceId") WHERE (("userId" IS NULL) AND ("applicationId" IS NULL));


--
-- Name: IDX_LOGIC_FUNCTION_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_LOGIC_FUNCTION_APPLICATION_ID" ON core."logicFunction" USING btree ("applicationId");


--
-- Name: IDX_LOGIC_FUNCTION_ID_DELETED_AT; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_LOGIC_FUNCTION_ID_DELETED_AT" ON core."logicFunction" USING btree (id, "deletedAt");


--
-- Name: IDX_MCP_SERVER_CONNECTION_PROVIDER_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_MCP_SERVER_CONNECTION_PROVIDER_ID" ON core."mcpServer" USING btree ("connectionProviderId");


--
-- Name: IDX_MCP_SERVER_WORKSPACE_ID_NAME_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_MCP_SERVER_WORKSPACE_ID_NAME_UNIQUE" ON core."mcpServer" USING btree ("workspaceId", name);


--
-- Name: IDX_MESSAGE_CHANNEL_APP_CONNECTED_ACCOUNT_HANDLE_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_MESSAGE_CHANNEL_APP_CONNECTED_ACCOUNT_HANDLE_UNIQUE" ON core."messageChannel" USING btree ("workspaceId", "connectedAccountId", handle) WHERE (type = 'APP'::core."messageChannel_type_enum");


--
-- Name: IDX_MESSAGE_CHANNEL_WEBHOOK_SUBSCRIPTION_EXTERNAL_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_MESSAGE_CHANNEL_WEBHOOK_SUBSCRIPTION_EXTERNAL_ID" ON core."messageChannel" USING btree ("webhookSubscriptionExternalId") WHERE ("webhookSubscriptionExternalId" IS NOT NULL);


--
-- Name: IDX_MESSAGE_CHANNEL_WORKSPACE_ID_SYNC_ENABLED_SYNC_STAGE; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_MESSAGE_CHANNEL_WORKSPACE_ID_SYNC_ENABLED_SYNC_STAGE" ON core."messageChannel" USING btree ("workspaceId", "isSyncEnabled", "syncStage");


--
-- Name: IDX_MESSAGE_FOLDER_MESSAGE_CHANNEL_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_MESSAGE_FOLDER_MESSAGE_CHANNEL_ID" ON core."messageFolder" USING btree ("messageChannelId");


--
-- Name: IDX_MESSAGE_FOLDER_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_MESSAGE_FOLDER_WORKSPACE_ID" ON core."messageFolder" USING btree ("workspaceId");


--
-- Name: IDX_MESSAGING_PROVIDER_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_MESSAGING_PROVIDER_APPLICATION_ID" ON core."messagingProvider" USING btree ("applicationId");


--
-- Name: IDX_NAVIGATION_MENU_ITEM_AGENT_CHAT_CHANNEL_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_AGENT_CHAT_CHANNEL_ID_WORKSPACE_ID" ON core."navigationMenuItem" USING btree ("agentChatChannelId", "workspaceId");


--
-- Name: IDX_NAVIGATION_MENU_ITEM_AGENT_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_AGENT_ID_WORKSPACE_ID" ON core."navigationMenuItem" USING btree ("agentId", "workspaceId");


--
-- Name: IDX_NAVIGATION_MENU_ITEM_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_APPLICATION_ID" ON core."navigationMenuItem" USING btree ("applicationId");


--
-- Name: IDX_NAVIGATION_MENU_ITEM_PAGE_LAYOUT_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_PAGE_LAYOUT_ID_WORKSPACE_ID" ON core."navigationMenuItem" USING btree ("pageLayoutId", "workspaceId");


--
-- Name: IDX_NAVIGATION_MENU_ITEM_PARENT_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_PARENT_ID_WORKSPACE_ID" ON core."navigationMenuItem" USING btree ("parentId", "workspaceId");


--
-- Name: IDX_NAVIGATION_MENU_ITEM_TARGET_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_TARGET_OBJECT_METADATA_ID" ON core."navigationMenuItem" USING btree ("targetObjectMetadataId") WHERE ("targetObjectMetadataId" IS NOT NULL);


--
-- Name: IDX_NAVIGATION_MENU_ITEM_TARGET_RECORD_OBJ_METADATA_WS_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_TARGET_RECORD_OBJ_METADATA_WS_ID" ON core."navigationMenuItem" USING btree ("targetRecordId", "targetObjectMetadataId", "workspaceId");


--
-- Name: IDX_NAVIGATION_MENU_ITEM_USER_WORKSPACE_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_USER_WORKSPACE_ID_WORKSPACE_ID" ON core."navigationMenuItem" USING btree ("userWorkspaceId", "workspaceId");


--
-- Name: IDX_NAVIGATION_MENU_ITEM_VIEW_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_VIEW_ID_WORKSPACE_ID" ON core."navigationMenuItem" USING btree ("viewId", "workspaceId");


--
-- Name: IDX_NAVIGATION_MENU_ITEM_WORKFLOW_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_NAVIGATION_MENU_ITEM_WORKFLOW_ID_WORKSPACE_ID" ON core."navigationMenuItem" USING btree ("workflowId", "workspaceId");


--
-- Name: IDX_OBJECT_METADATA_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_OBJECT_METADATA_APPLICATION_ID" ON core."objectMetadata" USING btree ("applicationId");


--
-- Name: IDX_OBJECT_PERMISSION_WORKSPACE_ID_ROLE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_OBJECT_PERMISSION_WORKSPACE_ID_ROLE_ID" ON core."objectPermission" USING btree ("workspaceId", "roleId");


--
-- Name: IDX_PAGE_LAYOUT_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_APPLICATION_ID" ON core."pageLayout" USING btree ("applicationId");


--
-- Name: IDX_PAGE_LAYOUT_DEFAULT_TAB_TO_FOCUS_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_DEFAULT_TAB_TO_FOCUS_ID" ON core."pageLayout" USING btree ("defaultTabToFocusOnMobileAndSidePanelId") WHERE ("defaultTabToFocusOnMobileAndSidePanelId" IS NOT NULL);


--
-- Name: IDX_PAGE_LAYOUT_OBJECT_METADATA_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_OBJECT_METADATA_ID_WORKSPACE_ID" ON core."pageLayout" USING btree ("objectMetadataId", "workspaceId") WHERE ("objectMetadataId" IS NOT NULL);


--
-- Name: IDX_PAGE_LAYOUT_TAB_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_TAB_APPLICATION_ID" ON core."pageLayoutTab" USING btree ("applicationId");


--
-- Name: IDX_PAGE_LAYOUT_TAB_PAGE_LAYOUT_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_TAB_PAGE_LAYOUT_ID" ON core."pageLayoutTab" USING btree ("pageLayoutId");


--
-- Name: IDX_PAGE_LAYOUT_TAB_WORKSPACE_ID_PAGE_LAYOUT_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_TAB_WORKSPACE_ID_PAGE_LAYOUT_ID" ON core."pageLayoutTab" USING btree ("workspaceId", "pageLayoutId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_PAGE_LAYOUT_WIDGET_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_WIDGET_APPLICATION_ID" ON core."pageLayoutWidget" USING btree ("applicationId");


--
-- Name: IDX_PAGE_LAYOUT_WIDGET_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_WIDGET_OBJECT_METADATA_ID" ON core."pageLayoutWidget" USING btree ("objectMetadataId");


--
-- Name: IDX_PAGE_LAYOUT_WIDGET_PAGE_LAYOUT_TAB_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_WIDGET_PAGE_LAYOUT_TAB_ID" ON core."pageLayoutWidget" USING btree ("pageLayoutTabId");


--
-- Name: IDX_PAGE_LAYOUT_WIDGET_WORKSPACE_ID_PAGE_LAYOUT_TAB_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PAGE_LAYOUT_WIDGET_WORKSPACE_ID_PAGE_LAYOUT_TAB_ID" ON core."pageLayoutWidget" USING btree ("workspaceId", "pageLayoutTabId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_PENDING_WAKE_UP_RESUME_AT; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PENDING_WAKE_UP_RESUME_AT" ON core."pendingWakeUp" USING btree ("resumeAt");


--
-- Name: IDX_PENDING_WAKE_UP_WORKSPACE_EVENT_NAME; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PENDING_WAKE_UP_WORKSPACE_EVENT_NAME" ON core."pendingWakeUp" USING btree ("workspaceId", "eventName");


--
-- Name: IDX_PERMISSION_FLAG_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PERMISSION_FLAG_APPLICATION_ID" ON core."permissionFlag" USING btree ("applicationId");


--
-- Name: IDX_PUBLIC_DOMAIN_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_PUBLIC_DOMAIN_APPLICATION_ID" ON core."publicDomain" USING btree ("applicationId");


--
-- Name: IDX_RELATED_RECORD_PATH_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RELATED_RECORD_PATH_APPLICATION_ID" ON core."relatedRecordPath" USING btree ("applicationId");


--
-- Name: IDX_RELATED_RECORD_PATH_FIRST_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RELATED_RECORD_PATH_FIRST_FIELD_METADATA_ID" ON core."relatedRecordPath" USING btree ("firstFieldMetadataId");


--
-- Name: IDX_RELATED_RECORD_PATH_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RELATED_RECORD_PATH_OBJECT_METADATA_ID" ON core."relatedRecordPath" USING btree ("objectMetadataId");


--
-- Name: IDX_RELATED_RECORD_PATH_SECOND_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RELATED_RECORD_PATH_SECOND_FIELD_METADATA_ID" ON core."relatedRecordPath" USING btree ("secondFieldMetadataId");


--
-- Name: IDX_RELATED_RECORD_PATH_THIRD_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RELATED_RECORD_PATH_THIRD_FIELD_METADATA_ID" ON core."relatedRecordPath" USING btree ("thirdFieldMetadataId");


--
-- Name: IDX_RELATED_RECORD_PATH_UNIQUE_ONE_HOP; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_RELATED_RECORD_PATH_UNIQUE_ONE_HOP" ON core."relatedRecordPath" USING btree ("firstFieldMetadataId") WHERE ("secondFieldMetadataId" IS NULL);


--
-- Name: IDX_RELATED_RECORD_PATH_UNIQUE_THREE_HOPS; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_RELATED_RECORD_PATH_UNIQUE_THREE_HOPS" ON core."relatedRecordPath" USING btree ("firstFieldMetadataId", "secondFieldMetadataId", "thirdFieldMetadataId") WHERE ("thirdFieldMetadataId" IS NOT NULL);


--
-- Name: IDX_RELATED_RECORD_PATH_UNIQUE_TWO_HOPS; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_RELATED_RECORD_PATH_UNIQUE_TWO_HOPS" ON core."relatedRecordPath" USING btree ("firstFieldMetadataId", "secondFieldMetadataId") WHERE (("secondFieldMetadataId" IS NOT NULL) AND ("thirdFieldMetadataId" IS NULL));


--
-- Name: IDX_RLPPG_PARENT_GROUP_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPPG_PARENT_GROUP_ID" ON core."rowLevelPermissionPredicateGroup" USING btree ("parentRowLevelPermissionPredicateGroupId");


--
-- Name: IDX_RLPPG_RULE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPPG_RULE_ID" ON core."rowLevelPermissionPredicateGroup" USING btree ("rowLevelPermissionRuleId");


--
-- Name: IDX_RLPPG_WORKSPACE_ID_ROLE_ID_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPPG_WORKSPACE_ID_ROLE_ID_OBJECT_METADATA_ID" ON core."rowLevelPermissionPredicateGroup" USING btree ("workspaceId", "roleId", "objectMetadataId");


--
-- Name: IDX_RLPP_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPP_FIELD_METADATA_ID" ON core."rowLevelPermissionPredicate" USING btree ("fieldMetadataId");


--
-- Name: IDX_RLPP_GROUP_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPP_GROUP_ID" ON core."rowLevelPermissionPredicate" USING btree ("rowLevelPermissionPredicateGroupId");


--
-- Name: IDX_RLPP_RELATION_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPP_RELATION_FIELD_METADATA_ID" ON core."rowLevelPermissionPredicate" USING btree ("relationFieldMetadataId");


--
-- Name: IDX_RLPP_RULE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPP_RULE_ID" ON core."rowLevelPermissionPredicate" USING btree ("rowLevelPermissionRuleId");


--
-- Name: IDX_RLPP_WORKSPACE_ID_ROLE_ID_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPP_WORKSPACE_ID_ROLE_ID_OBJECT_METADATA_ID" ON core."rowLevelPermissionPredicate" USING btree ("workspaceId", "roleId", "objectMetadataId");


--
-- Name: IDX_RLPP_WORKSPACE_MEMBER_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPP_WORKSPACE_MEMBER_FIELD_METADATA_ID" ON core."rowLevelPermissionPredicate" USING btree ("workspaceMemberFieldMetadataId");


--
-- Name: IDX_RLPP_WORKSPACE_MEMBER_RELATION_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPP_WORKSPACE_MEMBER_RELATION_FIELD_METADATA_ID" ON core."rowLevelPermissionPredicate" USING btree ("workspaceMemberRelationFieldMetadataId");


--
-- Name: IDX_RLPR_WORKSPACE_ID_ROLE_ID_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_RLPR_WORKSPACE_ID_ROLE_ID_OBJECT_METADATA_ID" ON core."rowLevelPermissionRule" USING btree ("workspaceId", "roleId", "objectMetadataId");


--
-- Name: IDX_ROLE_PERMISSION_FLAG_PERMISSION_FLAG_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_ROLE_PERMISSION_FLAG_PERMISSION_FLAG_ID" ON core."rolePermissionFlag" USING btree ("permissionFlagId");


--
-- Name: IDX_ROLE_PERMISSION_FLAG_ROLE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_ROLE_PERMISSION_FLAG_ROLE_ID" ON core."rolePermissionFlag" USING btree ("roleId");


--
-- Name: IDX_ROLE_TARGET_AGENT_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_ROLE_TARGET_AGENT_ID" ON core."roleTarget" USING btree ("agentId");


--
-- Name: IDX_ROLE_TARGET_API_KEY_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_ROLE_TARGET_API_KEY_ID" ON core."roleTarget" USING btree ("apiKeyId");


--
-- Name: IDX_ROLE_TARGET_ROLE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_ROLE_TARGET_ROLE_ID" ON core."roleTarget" USING btree ("roleId");


--
-- Name: IDX_ROLE_TARGET_TARGET_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_ROLE_TARGET_TARGET_APPLICATION_ID" ON core."roleTarget" USING btree ("targetApplicationId");


--
-- Name: IDX_ROLE_TARGET_UNIQUE_ASSIGNED_USER_WORKSPACE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_ROLE_TARGET_UNIQUE_ASSIGNED_USER_WORKSPACE" ON core."roleTarget" USING btree ("workspaceId", "userWorkspaceId") WHERE ("isSystemSideEffect" = false);


--
-- Name: IDX_ROLE_TARGET_UNIQUE_USER_WORKSPACE_ROLE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_ROLE_TARGET_UNIQUE_USER_WORKSPACE_ROLE" ON core."roleTarget" USING btree ("workspaceId", "userWorkspaceId", "roleId");


--
-- Name: IDX_ROLE_TARGET_WEBHOOK_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_ROLE_TARGET_WEBHOOK_ID" ON core."roleTarget" USING btree ("webhookId");


--
-- Name: IDX_ROLE_TARGET_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_ROLE_TARGET_WORKSPACE_ID" ON core."roleTarget" USING btree ("userWorkspaceId", "workspaceId");


--
-- Name: IDX_SEARCH_FIELD_METADATA_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_SEARCH_FIELD_METADATA_APPLICATION_ID" ON core."searchFieldMetadata" USING btree ("applicationId");


--
-- Name: IDX_SEARCH_FIELD_METADATA_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_SEARCH_FIELD_METADATA_OBJECT_METADATA_ID" ON core."searchFieldMetadata" USING btree ("objectMetadataId");


--
-- Name: IDX_SEARCH_FIELD_METADATA_TS_VECTOR_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_SEARCH_FIELD_METADATA_TS_VECTOR_FIELD_METADATA_ID" ON core."searchFieldMetadata" USING btree ("tsVectorFieldMetadataId");


--
-- Name: IDX_SETTINGS_MENU_ITEM_FRONT_COMPONENT_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_SETTINGS_MENU_ITEM_FRONT_COMPONENT_ID" ON core."settingsMenuItem" USING btree ("frontComponentId");


--
-- Name: IDX_SETTINGS_MENU_ITEM_WORKSPACE_ID_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_SETTINGS_MENU_ITEM_WORKSPACE_ID_APPLICATION_ID" ON core."settingsMenuItem" USING btree ("workspaceId", "applicationId");


--
-- Name: IDX_SIGNING_KEY_IS_CURRENT_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_SIGNING_KEY_IS_CURRENT_UNIQUE" ON core."signingKey" USING btree ("isCurrent") WHERE ("isCurrent" = true);


--
-- Name: IDX_SKILL_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_SKILL_APPLICATION_ID" ON core.skill USING btree ("applicationId");


--
-- Name: IDX_SKILL_ID_IS_ACTIVE; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_SKILL_ID_IS_ACTIVE" ON core.skill USING btree (id, "isActive");


--
-- Name: IDX_SKILL_NAME_WORKSPACE_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_SKILL_NAME_WORKSPACE_ID_UNIQUE" ON core.skill USING btree (name, "workspaceId") WHERE ("isActive" = true);


--
-- Name: IDX_TIMELINE_ACTIVITY_TYPE_BASE_EMIT_SLOT_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_TIMELINE_ACTIVITY_TYPE_BASE_EMIT_SLOT_UNIQUE" ON core."timelineActivityType" USING btree ("workspaceId", action, "objectUniversalIdentifier", "targetRelationFieldUniversalIdentifier") NULLS NOT DISTINCT WHERE ((action IS NOT NULL) AND ("replacesTimelineActivityTypeUniversalIdentifier" IS NULL));


--
-- Name: IDX_TIMELINE_ACTIVITY_TYPE_OVERRIDE_EMIT_SLOT_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_TIMELINE_ACTIVITY_TYPE_OVERRIDE_EMIT_SLOT_UNIQUE" ON core."timelineActivityType" USING btree ("workspaceId", action, "objectUniversalIdentifier", "targetRelationFieldUniversalIdentifier") NULLS NOT DISTINCT WHERE ((action IS NOT NULL) AND ("replacesTimelineActivityTypeUniversalIdentifier" IS NOT NULL));


--
-- Name: IDX_UNSUBSCRIBE_TOPIC_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_UNSUBSCRIBE_TOPIC_WORKSPACE_ID" ON core."unsubscribeTopic" USING btree ("workspaceId");


--
-- Name: IDX_UPGRADE_MIGRATION_WORKSPACE_ID_CREATED_AT; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_UPGRADE_MIGRATION_WORKSPACE_ID_CREATED_AT" ON core."upgradeMigration" USING btree ("workspaceId", "createdAt") WHERE ("workspaceId" IS NOT NULL);


--
-- Name: IDX_UPGRADE_MIGRATION_WORKSPACE_ID_NAME_ATTEMPT; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_UPGRADE_MIGRATION_WORKSPACE_ID_NAME_ATTEMPT" ON core."upgradeMigration" USING btree ("workspaceId", name, attempt) WHERE ("workspaceId" IS NOT NULL);


--
-- Name: IDX_USER_APPLICATION_VARIABLE_VALUE_USER_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_USER_APPLICATION_VARIABLE_VALUE_USER_WORKSPACE_ID" ON core."userApplicationVariableValue" USING btree ("userWorkspaceId");


--
-- Name: IDX_USER_APPLICATION_VARIABLE_VALUE_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_USER_APPLICATION_VARIABLE_VALUE_WORKSPACE_ID" ON core."userApplicationVariableValue" USING btree ("workspaceId");


--
-- Name: IDX_USER_SESSION_EXPIRES_AT; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_USER_SESSION_EXPIRES_AT" ON core."userSession" USING btree ("expiresAt");


--
-- Name: IDX_USER_SESSION_REVOKED_AT; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_USER_SESSION_REVOKED_AT" ON core."userSession" USING btree ("revokedAt");


--
-- Name: IDX_USER_SESSION_TOKEN_HASH_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_USER_SESSION_TOKEN_HASH_UNIQUE" ON core."userSession" USING btree ("tokenHash");


--
-- Name: IDX_USER_SESSION_USER_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_USER_SESSION_USER_ID" ON core."userSession" USING btree ("userId");


--
-- Name: IDX_USER_SESSION_USER_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_USER_SESSION_USER_WORKSPACE_ID" ON core."userSession" USING btree ("userWorkspaceId");


--
-- Name: IDX_USER_SESSION_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_USER_SESSION_WORKSPACE_ID" ON core."userSession" USING btree ("workspaceId");


--
-- Name: IDX_USER_WORKSPACE_USER_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_USER_WORKSPACE_USER_ID" ON core."userWorkspace" USING btree ("userId");


--
-- Name: IDX_USER_WORKSPACE_USER_ID_WORKSPACE_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_USER_WORKSPACE_USER_ID_WORKSPACE_ID_UNIQUE" ON core."userWorkspace" USING btree ("userId", "workspaceId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_USER_WORKSPACE_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_USER_WORKSPACE_WORKSPACE_ID" ON core."userWorkspace" USING btree ("workspaceId");


--
-- Name: IDX_VALIDATION_RULE_ERROR_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VALIDATION_RULE_ERROR_FIELD_METADATA_ID" ON core."validationRule" USING btree ("errorFieldMetadataId");


--
-- Name: IDX_VALIDATION_RULE_WORKSPACE_ID_OBJECT_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VALIDATION_RULE_WORKSPACE_ID_OBJECT_METADATA_ID" ON core."validationRule" USING btree ("workspaceId", "objectMetadataId");


--
-- Name: IDX_VIEW_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_APPLICATION_ID" ON core.view USING btree ("applicationId");


--
-- Name: IDX_VIEW_CALENDAR_END_FIELD_METADATA; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_CALENDAR_END_FIELD_METADATA" ON core.view USING btree ("calendarEndFieldMetadataId");


--
-- Name: IDX_VIEW_CALENDAR_FIELD_METADATA; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_CALENDAR_FIELD_METADATA" ON core.view USING btree ("calendarFieldMetadataId");


--
-- Name: IDX_VIEW_CREATED_BY_USER_WORKSPACE; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_CREATED_BY_USER_WORKSPACE" ON core.view USING btree ("createdByUserWorkspaceId");


--
-- Name: IDX_VIEW_FIELD_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FIELD_APPLICATION_ID" ON core."viewField" USING btree ("applicationId");


--
-- Name: IDX_VIEW_FIELD_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FIELD_FIELD_METADATA_ID" ON core."viewField" USING btree ("fieldMetadataId");


--
-- Name: IDX_VIEW_FIELD_FIELD_METADATA_ID_VIEW_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_VIEW_FIELD_FIELD_METADATA_ID_VIEW_ID_UNIQUE" ON core."viewField" USING btree ("fieldMetadataId", "viewId") WHERE (("deletedAt" IS NULL) AND ("isCanonical" = true));


--
-- Name: IDX_VIEW_FIELD_GROUP_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FIELD_GROUP_APPLICATION_ID" ON core."viewFieldGroup" USING btree ("applicationId");


--
-- Name: IDX_VIEW_FIELD_GROUP_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FIELD_GROUP_VIEW_ID" ON core."viewFieldGroup" USING btree ("viewId");


--
-- Name: IDX_VIEW_FIELD_GROUP_WORKSPACE_ID_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FIELD_GROUP_WORKSPACE_ID_VIEW_ID" ON core."viewFieldGroup" USING btree ("workspaceId", "viewId");


--
-- Name: IDX_VIEW_FIELD_RELATION_TARGET_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FIELD_RELATION_TARGET_FIELD_METADATA_ID" ON core."viewField" USING btree ("relationTargetFieldMetadataId") WHERE ("relationTargetFieldMetadataId" IS NOT NULL);


--
-- Name: IDX_VIEW_FIELD_VIEW_FIELD_GROUP_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FIELD_VIEW_FIELD_GROUP_ID" ON core."viewField" USING btree ("viewFieldGroupId");


--
-- Name: IDX_VIEW_FIELD_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FIELD_VIEW_ID" ON core."viewField" USING btree ("viewId");


--
-- Name: IDX_VIEW_FIELD_WORKSPACE_ID_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FIELD_WORKSPACE_ID_VIEW_ID" ON core."viewField" USING btree ("workspaceId", "viewId");


--
-- Name: IDX_VIEW_FILTER_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FILTER_FIELD_METADATA_ID" ON core."viewFilter" USING btree ("fieldMetadataId");


--
-- Name: IDX_VIEW_FILTER_GROUP_PARENT_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FILTER_GROUP_PARENT_ID" ON core."viewFilterGroup" USING btree ("parentViewFilterGroupId");


--
-- Name: IDX_VIEW_FILTER_GROUP_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FILTER_GROUP_VIEW_ID" ON core."viewFilterGroup" USING btree ("viewId");


--
-- Name: IDX_VIEW_FILTER_GROUP_WORKSPACE_ID_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FILTER_GROUP_WORKSPACE_ID_VIEW_ID" ON core."viewFilterGroup" USING btree ("workspaceId", "viewId");


--
-- Name: IDX_VIEW_FILTER_RELATION_TARGET_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FILTER_RELATION_TARGET_FIELD_METADATA_ID" ON core."viewFilter" USING btree ("relationTargetFieldMetadataId") WHERE ("relationTargetFieldMetadataId" IS NOT NULL);


--
-- Name: IDX_VIEW_FILTER_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FILTER_VIEW_ID" ON core."viewFilter" USING btree ("viewId");


--
-- Name: IDX_VIEW_FILTER_WORKSPACE_ID_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_FILTER_WORKSPACE_ID_VIEW_ID" ON core."viewFilter" USING btree ("workspaceId", "viewId");


--
-- Name: IDX_VIEW_GROUP_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_GROUP_APPLICATION_ID" ON core."viewGroup" USING btree ("applicationId");


--
-- Name: IDX_VIEW_GROUP_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_GROUP_VIEW_ID" ON core."viewGroup" USING btree ("viewId");


--
-- Name: IDX_VIEW_GROUP_WORKSPACE_ID_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_GROUP_WORKSPACE_ID_VIEW_ID" ON core."viewGroup" USING btree ("workspaceId", "viewId");


--
-- Name: IDX_VIEW_KANBAN_FIELD_METADATA; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_KANBAN_FIELD_METADATA" ON core.view USING btree ("kanbanAggregateOperationFieldMetadataId");


--
-- Name: IDX_VIEW_MAIN_GROUP_BY_FIELD_METADATA; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_MAIN_GROUP_BY_FIELD_METADATA" ON core.view USING btree ("mainGroupByFieldMetadataId");


--
-- Name: IDX_VIEW_OBJECT_METADATA_ID_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_OBJECT_METADATA_ID_WORKSPACE_ID" ON core.view USING btree ("objectMetadataId", "workspaceId");


--
-- Name: IDX_VIEW_SORT_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_SORT_FIELD_METADATA_ID" ON core."viewSort" USING btree ("fieldMetadataId");


--
-- Name: IDX_VIEW_SORT_FIELD_METADATA_ID_VIEW_ID_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_VIEW_SORT_FIELD_METADATA_ID_VIEW_ID_UNIQUE" ON core."viewSort" USING btree ("fieldMetadataId", "viewId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_VIEW_SORT_RELATION_TARGET_FIELD_METADATA_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_SORT_RELATION_TARGET_FIELD_METADATA_ID" ON core."viewSort" USING btree ("relationTargetFieldMetadataId") WHERE ("relationTargetFieldMetadataId" IS NOT NULL);


--
-- Name: IDX_VIEW_SORT_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_SORT_VIEW_ID" ON core."viewSort" USING btree ("viewId");


--
-- Name: IDX_VIEW_SORT_WORKSPACE_ID_VIEW_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_SORT_WORKSPACE_ID_VIEW_ID" ON core."viewSort" USING btree ("workspaceId", "viewId");


--
-- Name: IDX_VIEW_VISIBILITY; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_VIEW_VISIBILITY" ON core.view USING btree (visibility);


--
-- Name: IDX_WEBHOOK_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_WEBHOOK_WORKSPACE_ID" ON core.webhook USING btree ("workspaceId");


--
-- Name: IDX_WEB_PUSH_SUBSCRIPTION_USER_SESSION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_WEB_PUSH_SUBSCRIPTION_USER_SESSION_ID" ON core."webPushSubscription" USING btree ("userSessionId");


--
-- Name: IDX_WEB_PUSH_SUBSCRIPTION_WORKSPACE_ID_ENDPOINT_UNIQUE; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_WEB_PUSH_SUBSCRIPTION_WORKSPACE_ID_ENDPOINT_UNIQUE" ON core."webPushSubscription" USING btree ("workspaceId", endpoint);


--
-- Name: IDX_WORKFLOW_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_WORKFLOW_APPLICATION_ID" ON core.workflow USING btree ("applicationId");


--
-- Name: IDX_WORKFLOW_VERSION_APPLICATION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_WORKFLOW_VERSION_APPLICATION_ID" ON core."workflowVersion" USING btree ("applicationId");


--
-- Name: IDX_WORKFLOW_VERSION_ONE_ACTIVE_PER_WORKFLOW; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_WORKFLOW_VERSION_ONE_ACTIVE_PER_WORKFLOW" ON core."workflowVersion" USING btree ("workspaceId", "coreWorkflowId") WHERE (status = 'ACTIVE'::core."workflowVersion_status_enum");


--
-- Name: IDX_WORKFLOW_VERSION_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_WORKFLOW_VERSION_WORKSPACE_ID" ON core."workflowVersion" USING btree ("workspaceId");


--
-- Name: IDX_WORKFLOW_VERSION_WORKSPACE_VERSION_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_WORKFLOW_VERSION_WORKSPACE_VERSION_ID" ON core."workflowVersion" USING btree ("workspaceId", "workspaceWorkflowVersionId") WHERE ("workspaceWorkflowVersionId" IS NOT NULL);


--
-- Name: IDX_WORKFLOW_VISIBILITY_CREATED_BY; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_WORKFLOW_VISIBILITY_CREATED_BY" ON core.workflow USING btree ("workspaceId", visibility, "createdByUserWorkspaceId");


--
-- Name: IDX_WORKFLOW_WORKSPACE_ID; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_WORKFLOW_WORKSPACE_ID" ON core.workflow USING btree ("workspaceId");


--
-- Name: IDX_WORKSPACE_ACTIVATION_STATUS; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_WORKSPACE_ACTIVATION_STATUS" ON core.workspace USING btree ("activationStatus");


--
-- Name: IDX_a1413f7f0e71cb5825ac40c4fa; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_a1413f7f0e71cb5825ac40c4fa" ON core."frontComponent" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_a14b5665091e86d461fb585924; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_a14b5665091e86d461fb585924" ON core."rowLevelPermissionPredicateGroup" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_a3a5976e1b580ba1086c595802; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_a3a5976e1b580ba1086c595802" ON core."commandMenuItem" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_a44e3b03f0eca32d0504d5ef73; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_a44e3b03f0eca32d0504d5ef73" ON core."viewGroup" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_b27c681286ac581f81498c5d4b; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_b27c681286ac581f81498c5d4b" ON core."indexMetadata" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_b8282d1e10fbb7856950f86c61; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_b8282d1e10fbb7856950f86c61" ON core."twoFactorAuthenticationMethod" USING btree ("workspaceId");


--
-- Name: IDX_b86af4ea24cae518dee8eae996; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_b86af4ea24cae518dee8eae996" ON core."viewField" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_c28134c55ca1acff4e9d53d46e; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_c28134c55ca1acff4e9d53d46e" ON core."settingsMenuItem" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_c2e441c901b45221a70d325349; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_c2e441c901b45221a70d325349" ON core."searchFieldMetadata" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_c5ea53618b32558fe24e495f21; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_c5ea53618b32558fe24e495f21" ON core."objectPermission" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_c67a90cc29887078286205457e; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_c67a90cc29887078286205457e" ON core."workflowVersion" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_cc6e86ea81701c6d48ae239ea0; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_cc6e86ea81701c6d48ae239ea0" ON core."messagingProvider" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_cd4588bfc9ad73345b3953a039; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_cd4588bfc9ad73345b3953a039" ON core."viewFilter" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_d48d713d01cc3c81bad1f39795; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_d48d713d01cc3c81bad1f39795" ON core.webhook USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_d8cf7f15cf6466ac0e3b443b3d; Type: INDEX; Schema: core; Owner: -
--

CREATE INDEX "IDX_d8cf7f15cf6466ac0e3b443b3d" ON core."indexFieldMetadata" USING btree ("workspaceId");


--
-- Name: IDX_da8ffd3c24b4a819430a861067; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_da8ffd3c24b4a819430a861067" ON core."permissionFlag" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_e4559ae0dba56e53714137c704; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_e4559ae0dba56e53714137c704" ON core."rolePermissionFlag" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_e46f3e01227f1c8ee0c8041821; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_e46f3e01227f1c8ee0c8041821" ON core."rowLevelPermissionPredicate" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_e6398c21e6bb31b525272fac84; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_e6398c21e6bb31b525272fac84" ON core.skill USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_e6ed40a61e4584e98584019a47; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_e6ed40a61e4584e98584019a47" ON core."viewFilterGroup" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_e88d35604c4445b16e682edb30; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_e88d35604c4445b16e682edb30" ON core."viewFieldGroup" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_e92c299771228b9f31a092e64c; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_e92c299771228b9f31a092e64c" ON core."validationRule" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_f1c88fdfc3ad8910b17fc1fd73; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_f1c88fdfc3ad8910b17fc1fd73" ON core."fieldMetadata" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_f4a467e03f28dfc5578531295e; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_f4a467e03f28dfc5578531295e" ON core."mcpServer" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: IDX_f8ef066c77d63992fa0d3bf65b; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "IDX_f8ef066c77d63992fa0d3bf65b" ON core."agentChatChannelRole" USING btree ("workspaceId", "universalIdentifier");


--
-- Name: UQ_USER_EMAIL; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "UQ_USER_EMAIL" ON core."user" USING btree (email) WHERE ("deletedAt" IS NULL);


--
-- Name: UQ_upgrade_migration_instance; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "UQ_upgrade_migration_instance" ON core."upgradeMigration" USING btree (name, attempt) WHERE ("workspaceId" IS NULL);


--
-- Name: UQ_upgrade_migration_workspace; Type: INDEX; Schema: core; Owner: -
--

CREATE UNIQUE INDEX "UQ_upgrade_migration_workspace" ON core."upgradeMigration" USING btree (name, attempt, "workspaceId") WHERE ("workspaceId" IS NOT NULL);


--
-- Name: pageLayoutTab FK_0177b1574efe6e6f24651977340; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayoutTab"
    ADD CONSTRAINT "FK_0177b1574efe6e6f24651977340" FOREIGN KEY ("pageLayoutId") REFERENCES core."pageLayout"(id) ON DELETE CASCADE;


--
-- Name: applicationRegistrationVersion FK_02169996cebff7ce601ef1a8856; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistrationVersion"
    ADD CONSTRAINT "FK_02169996cebff7ce601ef1a8856" FOREIGN KEY ("applicationRegistrationId") REFERENCES core."applicationRegistration"(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_03c63a0b00ddc3ade21ed0b1a80; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_03c63a0b00ddc3ade21ed0b1a80" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: viewField FK_04182cf3303e503661915eb27ee; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewField"
    ADD CONSTRAINT "FK_04182cf3303e503661915eb27ee" FOREIGN KEY ("relationTargetFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: indexMetadata FK_051487e9b745cb175950130b63f; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."indexMetadata"
    ADD CONSTRAINT "FK_051487e9b745cb175950130b63f" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: agentChatChannelRole FK_05411d9312a897a828ad3e749ce; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."agentChatChannelRole"
    ADD CONSTRAINT "FK_05411d9312a897a828ad3e749ce" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: fieldMetadata FK_05453a954e458e3d91f2ff5043f; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadata"
    ADD CONSTRAINT "FK_05453a954e458e3d91f2ff5043f" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: indexMetadata FK_056363e1599f5b9a0e33323d9da; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."indexMetadata"
    ADD CONSTRAINT "FK_056363e1599f5b9a0e33323d9da" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_0626442171559be7dca10652215; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_0626442171559be7dca10652215" FOREIGN KEY ("agentChatChannelId") REFERENCES core."agentChatChannel"(id) ON DELETE CASCADE;


--
-- Name: pageLayoutWidget FK_0659a4d171c93f5c046f18d24cd; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayoutWidget"
    ADD CONSTRAINT "FK_0659a4d171c93f5c046f18d24cd" FOREIGN KEY ("pageLayoutTabId") REFERENCES core."pageLayoutTab"(id) ON DELETE CASCADE;


--
-- Name: settingsMenuItem FK_06777107034316d88f8f9deb058; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."settingsMenuItem"
    ADD CONSTRAINT "FK_06777107034316d88f8f9deb058" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: applicationRegistrationVariable FK_067a6267789011853178a6ab57a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistrationVariable"
    ADD CONSTRAINT "FK_067a6267789011853178a6ab57a" FOREIGN KEY ("applicationRegistrationId") REFERENCES core."applicationRegistration"(id) ON DELETE CASCADE;


--
-- Name: viewFilter FK_06858adf0fb54ec88fa602198ca; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilter"
    ADD CONSTRAINT "FK_06858adf0fb54ec88fa602198ca" FOREIGN KEY ("viewId") REFERENCES core.view(id) ON DELETE CASCADE;


--
-- Name: agentChatChannel FK_06abd1f6b49da7e9d621edbb34d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."agentChatChannel"
    ADD CONSTRAINT "FK_06abd1f6b49da7e9d621edbb34d" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: deferredWorkspaceOperation FK_06e51a1b028fc008516500af503; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."deferredWorkspaceOperation"
    ADD CONSTRAINT "FK_06e51a1b028fc008516500af503" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: application FK_08d1d5e33c2a3ce7c140e9b335b; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "FK_08d1d5e33c2a3ce7c140e9b335b" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;


--
-- Name: logicFunctionLayer FK_0a2947ca6a9adefa41eb62b2322; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."logicFunctionLayer"
    ADD CONSTRAINT "FK_0a2947ca6a9adefa41eb62b2322" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: viewField FK_0a48a0b66daedac1314437be5eb; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewField"
    ADD CONSTRAINT "FK_0a48a0b66daedac1314437be5eb" FOREIGN KEY ("fieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: keyValuePair FK_0dae35d1c0fbdda6495be4ae71a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."keyValuePair"
    ADD CONSTRAINT "FK_0dae35d1c0fbdda6495be4ae71a" FOREIGN KEY ("userId") REFERENCES core."user"(id) ON DELETE CASCADE;


--
-- Name: settingsMenuItem FK_110a68950a78f6b30316cad894f; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."settingsMenuItem"
    ADD CONSTRAINT "FK_110a68950a78f6b30316cad894f" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: viewFieldGroup FK_118208b32ebf53be5aaede9c9cf; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFieldGroup"
    ADD CONSTRAINT "FK_118208b32ebf53be5aaede9c9cf" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: viewSort FK_128973737fae365a2db6bba3c49; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewSort"
    ADD CONSTRAINT "FK_128973737fae365a2db6bba3c49" FOREIGN KEY ("relationTargetFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE SET NULL;


--
-- Name: fieldMetadataOption FK_12966faa34751ce8979af6e5a1a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadataOption"
    ADD CONSTRAINT "FK_12966faa34751ce8979af6e5a1a" FOREIGN KEY ("fieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_15199deab40d48dd1480a2faf85; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_15199deab40d48dd1480a2faf85" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: pendingWakeUp FK_15bba3e321919925d62262a14a2; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pendingWakeUp"
    ADD CONSTRAINT "FK_15bba3e321919925d62262a14a2" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: viewFieldGroup FK_15c7197294c08e6e780d9734c99; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFieldGroup"
    ADD CONSTRAINT "FK_15c7197294c08e6e780d9734c99" FOREIGN KEY ("viewId") REFERENCES core.view(id) ON DELETE CASCADE;


--
-- Name: roleTarget FK_16433a32ab13a294569e52a10e0; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "FK_16433a32ab13a294569e52a10e0" FOREIGN KEY ("agentId") REFERENCES core.agent(id) ON DELETE CASCADE;


--
-- Name: unsubscribeTopic FK_16d7bf6f90fac4745c89e1e8d56; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."unsubscribeTopic"
    ADD CONSTRAINT "FK_16d7bf6f90fac4745c89e1e8d56" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: connectionProvider FK_16d8e4d029dd986268d759a2257; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectionProvider"
    ADD CONSTRAINT "FK_16d8e4d029dd986268d759a2257" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_18554ded14c33fc6f075922ef95; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_18554ded14c33fc6f075922ef95" FOREIGN KEY ("relationFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: viewFilter FK_193548db5abc45713087f7d1af6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilter"
    ADD CONSTRAINT "FK_193548db5abc45713087f7d1af6" FOREIGN KEY ("fieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: searchFieldMetadata FK_1b78544eb06f82059a2a01013a3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."searchFieldMetadata"
    ADD CONSTRAINT "FK_1b78544eb06f82059a2a01013a3" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;


--
-- Name: connectedAccount FK_1c7af038a011e99c27044793c6a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectedAccount"
    ADD CONSTRAINT "FK_1c7af038a011e99c27044793c6a" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicateGroup FK_1e82563accb67114f65a3993b86; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicateGroup"
    ADD CONSTRAINT "FK_1e82563accb67114f65a3993b86" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: connectedAccount FK_21b8e7d3a21ff5712c4dd4875ac; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectedAccount"
    ADD CONSTRAINT "FK_21b8e7d3a21ff5712c4dd4875ac" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: messageChannel FK_22d9a21a23fdd99295dc0efc177; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messageChannel"
    ADD CONSTRAINT "FK_22d9a21a23fdd99295dc0efc177" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: userWorkspace FK_22f5e76f493c3fb20237cfc48b0; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userWorkspace"
    ADD CONSTRAINT "FK_22f5e76f493c3fb20237cfc48b0" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_23b36d07d363f81200654fa1334; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_23b36d07d363f81200654fa1334" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: pageLayoutTab FK_2528e67c8c0c953d8303172989e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayoutTab"
    ADD CONSTRAINT "FK_2528e67c8c0c953d8303172989e" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: agent FK_259c48f99f625708723414adb5d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.agent
    ADD CONSTRAINT "FK_259c48f99f625708723414adb5d" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicateGroup FK_25bbd97a29478e18061cb58950f; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicateGroup"
    ADD CONSTRAINT "FK_25bbd97a29478e18061cb58950f" FOREIGN KEY ("parentRowLevelPermissionPredicateGroupId") REFERENCES core."rowLevelPermissionPredicateGroup"(id) ON DELETE CASCADE;


--
-- Name: usageLimit FK_262e7eabfa66b9724f7bf45628e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."usageLimit"
    ADD CONSTRAINT "FK_262e7eabfa66b9724f7bf45628e" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: fieldPermission FK_2763aee5614b54019d692333fe1; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldPermission"
    ADD CONSTRAINT "FK_2763aee5614b54019d692333fe1" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: fieldMetadataOption FK_27b4d06b908e0663b675de3e974; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadataOption"
    ADD CONSTRAINT "FK_27b4d06b908e0663b675de3e974" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: workspace FK_282123b2f32e927b6003311e33a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workspace
    ADD CONSTRAINT "FK_282123b2f32e927b6003311e33a" FOREIGN KEY ("logoFileId") REFERENCES core.file(id) ON DELETE SET NULL;


--
-- Name: application FK_28f20711184b3c3318a8e44d117; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "FK_28f20711184b3c3318a8e44d117" FOREIGN KEY ("yarnLockFileId") REFERENCES core.file(id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;


--
-- Name: commandMenuItem FK_29d15e273efc5804c653dc5ab93; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."commandMenuItem"
    ADD CONSTRAINT "FK_29d15e273efc5804c653dc5ab93" FOREIGN KEY ("navigationTargetObjectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: workflowVersion FK_29f62766c5b109981244b97060d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."workflowVersion"
    ADD CONSTRAINT "FK_29f62766c5b109981244b97060d" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: viewSort FK_2b36c6adea4542b4844d9fb1806; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewSort"
    ADD CONSTRAINT "FK_2b36c6adea4542b4844d9fb1806" FOREIGN KEY ("viewId") REFERENCES core.view(id) ON DELETE CASCADE;


--
-- Name: timelineActivityType FK_2c7b4a18f54e8da1956a7a7dd60; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."timelineActivityType"
    ADD CONSTRAINT "FK_2c7b4a18f54e8da1956a7a7dd60" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_2d41ba0f36326735cb9687456fc; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_2d41ba0f36326735cb9687456fc" FOREIGN KEY ("workflowId") REFERENCES core.workflow(id) ON DELETE CASCADE;


--
-- Name: viewGroup FK_2d7cfc4748058a0ca648835d046; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewGroup"
    ADD CONSTRAINT "FK_2d7cfc4748058a0ca648835d046" FOREIGN KEY ("viewId") REFERENCES core.view(id) ON DELETE CASCADE;


--
-- Name: messageChannel FK_2e966cbb240771c67630d52895c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messageChannel"
    ADD CONSTRAINT "FK_2e966cbb240771c67630d52895c" FOREIGN KEY ("connectedAccountId") REFERENCES core."connectedAccount"(id) ON DELETE CASCADE;


--
-- Name: searchFieldMetadata FK_31102b31f7338b6a298fc22996a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."searchFieldMetadata"
    ADD CONSTRAINT "FK_31102b31f7338b6a298fc22996a" FOREIGN KEY ("tsVectorFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;


--
-- Name: agentChatChannelRole FK_312b5b71997bc0556d4f8c489c6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."agentChatChannelRole"
    ADD CONSTRAINT "FK_312b5b71997bc0556d4f8c489c6" FOREIGN KEY ("channelId") REFERENCES core."agentChatChannel"(id) ON DELETE CASCADE;


--
-- Name: viewFilter FK_32cabc67e40d24acab541c469a8; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilter"
    ADD CONSTRAINT "FK_32cabc67e40d24acab541c469a8" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: commandMenuItem FK_342086f37d44e726a359ed6fd7d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."commandMenuItem"
    ADD CONSTRAINT "FK_342086f37d44e726a359ed6fd7d" FOREIGN KEY ("frontComponentId") REFERENCES core."frontComponent"(id) ON DELETE CASCADE;


--
-- Name: view FK_348e25d584c7e51417f4e097941; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.view
    ADD CONSTRAINT "FK_348e25d584c7e51417f4e097941" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: applicationRegistration FK_36715821de396df9536fd4afc81; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistration"
    ADD CONSTRAINT "FK_36715821de396df9536fd4afc81" FOREIGN KEY ("tarballFileId") REFERENCES core.file(id) ON DELETE SET NULL;


--
-- Name: application FK_3818380258798f9ffa9963b6dc4; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "FK_3818380258798f9ffa9963b6dc4" FOREIGN KEY ("packageJsonFileId") REFERENCES core.file(id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;


--
-- Name: rolePermissionFlag FK_3835ecc1019327566d35728c8ba; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rolePermissionFlag"
    ADD CONSTRAINT "FK_3835ecc1019327566d35728c8ba" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: application FK_38ac5dccee353ca07862a5a94bf; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "FK_38ac5dccee353ca07862a5a94bf" FOREIGN KEY ("primaryPublicDomainId") REFERENCES core."publicDomain"(id) ON DELETE SET NULL;


--
-- Name: viewFieldGroup FK_38ec9201914a42386e5cdaa6521; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFieldGroup"
    ADD CONSTRAINT "FK_38ec9201914a42386e5cdaa6521" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: view FK_394132f681ecbffa8ac912d1e5f; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.view
    ADD CONSTRAINT "FK_394132f681ecbffa8ac912d1e5f" FOREIGN KEY ("createdByUserWorkspaceId") REFERENCES core."userWorkspace"(id) ON DELETE SET NULL;


--
-- Name: publicDomain FK_39f1ad35993f3994cd5400e81a0; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."publicDomain"
    ADD CONSTRAINT "FK_39f1ad35993f3994cd5400e81a0" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: workspace FK_3b1acb13a5dac9956d1a4b32755; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workspace
    ADD CONSTRAINT "FK_3b1acb13a5dac9956d1a4b32755" FOREIGN KEY ("workspaceCustomApplicationId") REFERENCES core.application(id) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED;


--
-- Name: application FK_3d6ee2b75b81933c1708918f647; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "FK_3d6ee2b75b81933c1708918f647" FOREIGN KEY ("logoFileId") REFERENCES core.file(id) ON DELETE SET NULL;


--
-- Name: view FK_3e5ea41c239ef1b75b0d42bef99; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.view
    ADD CONSTRAINT "FK_3e5ea41c239ef1b75b0d42bef99" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicateGroup FK_3f1abc7557a4e9f4334d41b07d7; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicateGroup"
    ADD CONSTRAINT "FK_3f1abc7557a4e9f4334d41b07d7" FOREIGN KEY ("roleId") REFERENCES core.role(id) ON DELETE CASCADE;


--
-- Name: roleTarget FK_3f78f147358cb6d12a82ec02a96; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "FK_3f78f147358cb6d12a82ec02a96" FOREIGN KEY ("targetApplicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: connectedAccount FK_40de45e67a285dafb84e510cdc6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectedAccount"
    ADD CONSTRAINT "FK_40de45e67a285dafb84e510cdc6" FOREIGN KEY ("connectionProviderId") REFERENCES core."connectionProvider"(id) ON DELETE CASCADE;


--
-- Name: file FK_413aaaf293284c3c0266d0bab3a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.file
    ADD CONSTRAINT "FK_413aaaf293284c3c0266d0bab3a" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE RESTRICT;


--
-- Name: messageFolder FK_4237a2fe8a6583354f807c2f8fe; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messageFolder"
    ADD CONSTRAINT "FK_4237a2fe8a6583354f807c2f8fe" FOREIGN KEY ("messageChannelId") REFERENCES core."messageChannel"(id) ON DELETE CASCADE;


--
-- Name: workflowVersion FK_4316468725741b8e11e02b144f3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."workflowVersion"
    ADD CONSTRAINT "FK_4316468725741b8e11e02b144f3" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: pageLayoutTab FK_4493447c2e4029aa26cabf30460; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayoutTab"
    ADD CONSTRAINT "FK_4493447c2e4029aa26cabf30460" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: skill FK_46f69b93b58666bb388c5c7785a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.skill
    ADD CONSTRAINT "FK_46f69b93b58666bb388c5c7785a" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: fieldMetadata FK_47a6c57e1652b6475f8248cff78; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadata"
    ADD CONSTRAINT "FK_47a6c57e1652b6475f8248cff78" FOREIGN KEY ("relationTargetFieldMetadataId") REFERENCES core."fieldMetadata"(id) DEFERRABLE INITIALLY DEFERRED;


--
-- Name: relatedRecordPath FK_49d185010a658fe0c23d90ee658; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."relatedRecordPath"
    ADD CONSTRAINT "FK_49d185010a658fe0c23d90ee658" FOREIGN KEY ("firstFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: roleTarget FK_4b3865868c7da0747ee8e480851; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "FK_4b3865868c7da0747ee8e480851" FOREIGN KEY ("apiKeyId") REFERENCES core."apiKey"(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_4ba3e5e988c4c5f159ec8753ee3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_4ba3e5e988c4c5f159ec8753ee3" FOREIGN KEY ("pageLayoutId") REFERENCES core."pageLayout"(id) ON DELETE CASCADE;


--
-- Name: rolePermissionFlag FK_4c6ea38698de230b0ec18fa2110; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rolePermissionFlag"
    ADD CONSTRAINT "FK_4c6ea38698de230b0ec18fa2110" FOREIGN KEY ("roleId") REFERENCES core.role(id) ON DELETE CASCADE;


--
-- Name: fieldPermission FK_50aa5710a4ad47709a841d60364; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldPermission"
    ADD CONSTRAINT "FK_50aa5710a4ad47709a841d60364" FOREIGN KEY ("rowLevelPermissionPredicateGroupId") REFERENCES core."rowLevelPermissionPredicateGroup"(id) ON DELETE CASCADE;


--
-- Name: applicationVariable FK_51adb49e7f8df35dd23e01c4830; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationVariable"
    ADD CONSTRAINT "FK_51adb49e7f8df35dd23e01c4830" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionRule FK_53ebd0f6b6a09ba95da5b75db23; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionRule"
    ADD CONSTRAINT "FK_53ebd0f6b6a09ba95da5b75db23" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: pageLayoutWidget FK_555948f84165dce1fe1f5f955ce; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayoutWidget"
    ADD CONSTRAINT "FK_555948f84165dce1fe1f5f955ce" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: view FK_580dad12c8b92f3a3c307c4e66d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.view
    ADD CONSTRAINT "FK_580dad12c8b92f3a3c307c4e66d" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: webhook FK_597ab5e7de76f1836b8fd80d6b9; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.webhook
    ADD CONSTRAINT "FK_597ab5e7de76f1836b8fd80d6b9" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: agentChatChannelRole FK_5acd9b776da2f6cc9813d416c23; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."agentChatChannelRole"
    ADD CONSTRAINT "FK_5acd9b776da2f6cc9813d416c23" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: viewGroup FK_5aff384532c78fa8a42ceeae282; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewGroup"
    ADD CONSTRAINT "FK_5aff384532c78fa8a42ceeae282" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: userApplicationVariableValue FK_5b5a27417740e6bb8f1cb4b8fce; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userApplicationVariableValue"
    ADD CONSTRAINT "FK_5b5a27417740e6bb8f1cb4b8fce" FOREIGN KEY ("userWorkspaceId") REFERENCES core."userWorkspace"(id) ON DELETE CASCADE;


--
-- Name: view FK_5c0d21d6b8d5544a24ab9787114; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.view
    ADD CONSTRAINT "FK_5c0d21d6b8d5544a24ab9787114" FOREIGN KEY ("calendarFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: indexMetadata FK_5c988136a6d6f25a100c1064789; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."indexMetadata"
    ADD CONSTRAINT "FK_5c988136a6d6f25a100c1064789" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: pageLayout FK_5e7f19b88c0864db19e2bad0fc5; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayout"
    ADD CONSTRAINT "FK_5e7f19b88c0864db19e2bad0fc5" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: relatedRecordPath FK_5edcdacfe9a320166f833bd951d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."relatedRecordPath"
    ADD CONSTRAINT "FK_5edcdacfe9a320166f833bd951d" FOREIGN KEY ("thirdFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: searchFieldMetadata FK_5f10e00da471e19f52513f47d8b; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."searchFieldMetadata"
    ADD CONSTRAINT "FK_5f10e00da471e19f52513f47d8b" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: viewSort FK_5f3278d6791aa4c58423e556ae6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewSort"
    ADD CONSTRAINT "FK_5f3278d6791aa4c58423e556ae6" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_5fb4b0cebaf1b6418412bf65170; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_5fb4b0cebaf1b6418412bf65170" FOREIGN KEY ("roleId") REFERENCES core.role(id) ON DELETE CASCADE;


--
-- Name: agentChatChannel FK_60b86d8db38bff7ebf4b4e3167d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."agentChatChannel"
    ADD CONSTRAINT "FK_60b86d8db38bff7ebf4b4e3167d" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: viewGroup FK_61053f5509cc31e5d7139fba1cb; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewGroup"
    ADD CONSTRAINT "FK_61053f5509cc31e5d7139fba1cb" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: relatedRecordPath FK_623b51b39f35e8166ab756cf516; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."relatedRecordPath"
    ADD CONSTRAINT "FK_623b51b39f35e8166ab756cf516" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_62d47d14b50b67a03f832481de7; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_62d47d14b50b67a03f832481de7" FOREIGN KEY ("targetObjectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: applicationTranslation FK_62da06a264eae9a1e84f6c611bd; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationTranslation"
    ADD CONSTRAINT "FK_62da06a264eae9a1e84f6c611bd" FOREIGN KEY ("applicationRegistrationId") REFERENCES core."applicationRegistration"(id) ON DELETE CASCADE;


--
-- Name: frontComponent FK_63e430d5f8e554c4282e7b48876; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."frontComponent"
    ADD CONSTRAINT "FK_63e430d5f8e554c4282e7b48876" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_67519abd77fc637444720192737; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_67519abd77fc637444720192737" FOREIGN KEY ("rowLevelPermissionPredicateGroupId") REFERENCES core."rowLevelPermissionPredicateGroup"(id) ON DELETE CASCADE;


--
-- Name: workflow FK_6819d862ed54fbf00cecaa0da4b; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workflow
    ADD CONSTRAINT "FK_6819d862ed54fbf00cecaa0da4b" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: messagingProvider FK_6a936124386fc95f14ba5959ff0; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messagingProvider"
    ADD CONSTRAINT "FK_6a936124386fc95f14ba5959ff0" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: viewFilterGroup FK_6aa17342705ae5526de377bf7ed; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilterGroup"
    ADD CONSTRAINT "FK_6aa17342705ae5526de377bf7ed" FOREIGN KEY ("parentViewFilterGroupId") REFERENCES core."viewFilterGroup"(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;


--
-- Name: validationRule FK_6b3d85b4b9f0529f24e43649d4c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."validationRule"
    ADD CONSTRAINT "FK_6b3d85b4b9f0529f24e43649d4c" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: featureFlag FK_6be7761fa8453f3a498aab6e72b; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."featureFlag"
    ADD CONSTRAINT "FK_6be7761fa8453f3a498aab6e72b" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: searchFieldMetadata FK_6d5c6922bfd1578b1eff2abb9d6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."searchFieldMetadata"
    ADD CONSTRAINT "FK_6d5c6922bfd1578b1eff2abb9d6" FOREIGN KEY ("fieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;


--
-- Name: commandMenuItem FK_6e050fb56a8385718123a4f8bc6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."commandMenuItem"
    ADD CONSTRAINT "FK_6e050fb56a8385718123a4f8bc6" FOREIGN KEY ("availabilityObjectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: validationRule FK_6eed71e19346f9e09f1fa94ed76; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."validationRule"
    ADD CONSTRAINT "FK_6eed71e19346f9e09f1fa94ed76" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: fieldMetadata FK_6f6c87ec32cca956d8be321071c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadata"
    ADD CONSTRAINT "FK_6f6c87ec32cca956d8be321071c" FOREIGN KEY ("relationTargetObjectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_6fd84a774fe4ea4daa9aeeee5ed; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_6fd84a774fe4ea4daa9aeeee5ed" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: userApplicationVariableValue FK_71281400f08a8466962c3f067e0; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userApplicationVariableValue"
    ADD CONSTRAINT "FK_71281400f08a8466962c3f067e0" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: objectMetadata FK_71a7af5a5c916f0b96f358f25f7; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectMetadata"
    ADD CONSTRAINT "FK_71a7af5a5c916f0b96f358f25f7" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: fieldPermission FK_71cc60c4a1c9f8a7c434d91d38c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldPermission"
    ADD CONSTRAINT "FK_71cc60c4a1c9f8a7c434d91d38c" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: approvedAccessDomain FK_73d3e340b6ce0716a25a86361fc; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."approvedAccessDomain"
    ADD CONSTRAINT "FK_73d3e340b6ce0716a25a86361fc" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: connectionProviderOAuthClient FK_743610267fa3f2e1bfd67f75cf3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectionProviderOAuthClient"
    ADD CONSTRAINT "FK_743610267fa3f2e1bfd67f75cf3" FOREIGN KEY ("connectionProviderId") REFERENCES core."connectionProvider"(id) ON DELETE CASCADE;


--
-- Name: pageLayout FK_747fbc25827bdcb9e35cc68a990; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayout"
    ADD CONSTRAINT "FK_747fbc25827bdcb9e35cc68a990" FOREIGN KEY ("defaultTabToFocusOnMobileAndSidePanelId") REFERENCES core."pageLayoutTab"(id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED;


--
-- Name: mcpServer FK_756acd9aa1352b9c60128bec232; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."mcpServer"
    ADD CONSTRAINT "FK_756acd9aa1352b9c60128bec232" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionRule FK_75fe6c127241395429e691cc09c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionRule"
    ADD CONSTRAINT "FK_75fe6c127241395429e691cc09c" FOREIGN KEY ("roleId") REFERENCES core.role(id) ON DELETE CASCADE;


--
-- Name: pageLayout FK_760ec8b78721991220b76accd55; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayout"
    ADD CONSTRAINT "FK_760ec8b78721991220b76accd55" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: upgradeMigration FK_77f64a697c55f8802592bd7eeba; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."upgradeMigration"
    ADD CONSTRAINT "FK_77f64a697c55f8802592bd7eeba" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: applicationVariable FK_78ae6cfe5f49a76c4bf842ad58b; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationVariable"
    ADD CONSTRAINT "FK_78ae6cfe5f49a76c4bf842ad58b" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: emailingDomain FK_793a938bef2aae0a2129f78951f; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."emailingDomain"
    ADD CONSTRAINT "FK_793a938bef2aae0a2129f78951f" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: applicationRegistration FK_796819fb23559c233e6ebd49f34; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistration"
    ADD CONSTRAINT "FK_796819fb23559c233e6ebd49f34" FOREIGN KEY ("logoFileId") REFERENCES core.file(id) ON DELETE SET NULL;


--
-- Name: publicDomain FK_7e9ca5fd7aa30b8396ea3d1d6be; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."publicDomain"
    ADD CONSTRAINT "FK_7e9ca5fd7aa30b8396ea3d1d6be" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: role FK_7f3b96f15aaf5a27549288d264b; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.role
    ADD CONSTRAINT "FK_7f3b96f15aaf5a27549288d264b" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: viewSort FK_818522b962a9b756accb5b3149d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewSort"
    ADD CONSTRAINT "FK_818522b962a9b756accb5b3149d" FOREIGN KEY ("fieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_820da06779b7311c63bbe8386af; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_820da06779b7311c63bbe8386af" FOREIGN KEY ("workspaceMemberRelationFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: objectPermission FK_826052747c82e59f0a006204256; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectPermission"
    ADD CONSTRAINT "FK_826052747c82e59f0a006204256" FOREIGN KEY ("roleId") REFERENCES core.role(id) ON DELETE CASCADE;


--
-- Name: permissionFlag FK_835bc9f7ef959debfc5cd268049; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."permissionFlag"
    ADD CONSTRAINT "FK_835bc9f7ef959debfc5cd268049" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: roleTarget FK_83ea4a0433da5007a198db7667e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "FK_83ea4a0433da5007a198db7667e" FOREIGN KEY ("roleId") REFERENCES core.role(id) ON DELETE CASCADE;


--
-- Name: settingsMenuItem FK_8464a234a1389b2c9783408d210; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."settingsMenuItem"
    ADD CONSTRAINT "FK_8464a234a1389b2c9783408d210" FOREIGN KEY ("frontComponentId") REFERENCES core."frontComponent"(id) ON DELETE CASCADE;


--
-- Name: commandMenuItem FK_8577be6253969364b6725b807b4; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."commandMenuItem"
    ADD CONSTRAINT "FK_8577be6253969364b6725b807b4" FOREIGN KEY ("pageLayoutId") REFERENCES core."pageLayout"(id) ON DELETE CASCADE;


--
-- Name: rolePermissionFlag FK_8724e63323f1331591a3e91b0b3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rolePermissionFlag"
    ADD CONSTRAINT "FK_8724e63323f1331591a3e91b0b3" FOREIGN KEY ("permissionFlagId") REFERENCES core."permissionFlag"(id) ON DELETE CASCADE;


--
-- Name: messagingProvider FK_877a5039c291fe9b340d79c84fa; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messagingProvider"
    ADD CONSTRAINT "FK_877a5039c291fe9b340d79c84fa" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: viewFilterGroup FK_8919a390f4022ab1e40182a5ac3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilterGroup"
    ADD CONSTRAINT "FK_8919a390f4022ab1e40182a5ac3" FOREIGN KEY ("viewId") REFERENCES core.view(id) ON DELETE CASCADE;


--
-- Name: relatedRecordPath FK_8c0230764deef4970404480a259; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."relatedRecordPath"
    ADD CONSTRAINT "FK_8c0230764deef4970404480a259" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: appToken FK_8cd4819144baf069777b5729136; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."appToken"
    ADD CONSTRAINT "FK_8cd4819144baf069777b5729136" FOREIGN KEY ("userId") REFERENCES core."user"(id) ON DELETE CASCADE;


--
-- Name: searchFieldMetadata FK_927b6101a5d9562a558a18ed412; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."searchFieldMetadata"
    ADD CONSTRAINT "FK_927b6101a5d9562a558a18ed412" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: commandMenuItem FK_94947770f00413f134a1ec01dd7; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."commandMenuItem"
    ADD CONSTRAINT "FK_94947770f00413f134a1ec01dd7" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: applicationRegistration FK_94ab20372e448d45088357f884e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistration"
    ADD CONSTRAINT "FK_94ab20372e448d45088357f884e" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE SET NULL;


--
-- Name: viewField FK_96158de54c78944b5340b6f708e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewField"
    ADD CONSTRAINT "FK_96158de54c78944b5340b6f708e" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: relatedRecordPath FK_9caff61edf6734e6ec7e08d7863; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."relatedRecordPath"
    ADD CONSTRAINT "FK_9caff61edf6734e6ec7e08d7863" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: fieldMetadata FK_9ce5ba7878f498bcf79e447a9a6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadata"
    ADD CONSTRAINT "FK_9ce5ba7878f498bcf79e447a9a6" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: timelineActivityType FK_9dff241d7d796da9f63a8ae4bc6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."timelineActivityType"
    ADD CONSTRAINT "FK_9dff241d7d796da9f63a8ae4bc6" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_9ec9d8bc9bb4197be12d4efcaf3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_9ec9d8bc9bb4197be12d4efcaf3" FOREIGN KEY ("viewId") REFERENCES core.view(id) ON DELETE CASCADE;


--
-- Name: agent FK_AGENT_CREATED_BY_USER_WORKSPACE; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.agent
    ADD CONSTRAINT "FK_AGENT_CREATED_BY_USER_WORKSPACE" FOREIGN KEY ("createdByUserWorkspaceId") REFERENCES core."userWorkspace"(id) ON DELETE SET NULL;


--
-- Name: applicationAuthorization FK_APPLICATION_AUTHORIZATION_APPLICATION_ID; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationAuthorization"
    ADD CONSTRAINT "FK_APPLICATION_AUTHORIZATION_APPLICATION_ID" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: applicationAuthorization FK_APPLICATION_AUTHORIZATION_USER_ID; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationAuthorization"
    ADD CONSTRAINT "FK_APPLICATION_AUTHORIZATION_USER_ID" FOREIGN KEY ("userId") REFERENCES core."user"(id) ON DELETE CASCADE;


--
-- Name: applicationAuthorization FK_APPLICATION_AUTHORIZATION_USER_WORKSPACE_ID; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationAuthorization"
    ADD CONSTRAINT "FK_APPLICATION_AUTHORIZATION_USER_WORKSPACE_ID" FOREIGN KEY ("userWorkspaceId") REFERENCES core."userWorkspace"(id) ON DELETE CASCADE;


--
-- Name: applicationAuthorization FK_APPLICATION_AUTHORIZATION_WORKSPACE_ID; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationAuthorization"
    ADD CONSTRAINT "FK_APPLICATION_AUTHORIZATION_WORKSPACE_ID" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: userSession FK_USER_SESSION_USER_ID; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userSession"
    ADD CONSTRAINT "FK_USER_SESSION_USER_ID" FOREIGN KEY ("userId") REFERENCES core."user"(id) ON DELETE CASCADE;


--
-- Name: userSession FK_USER_SESSION_USER_WORKSPACE_ID; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userSession"
    ADD CONSTRAINT "FK_USER_SESSION_USER_WORKSPACE_ID" FOREIGN KEY ("userWorkspaceId") REFERENCES core."userWorkspace"(id) ON DELETE CASCADE;


--
-- Name: userSession FK_USER_SESSION_WORKSPACE_ID; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userSession"
    ADD CONSTRAINT "FK_USER_SESSION_WORKSPACE_ID" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: workflow FK_WORKFLOW_CREATED_BY_USER_WORKSPACE; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workflow
    ADD CONSTRAINT "FK_WORKFLOW_CREATED_BY_USER_WORKSPACE" FOREIGN KEY ("createdByUserWorkspaceId") REFERENCES core."userWorkspace"(id) ON DELETE SET NULL;


--
-- Name: connectionProvider FK_a2553b431536a5b93211012f984; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectionProvider"
    ADD CONSTRAINT "FK_a2553b431536a5b93211012f984" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: userWorkspace FK_a2da2ea7d6cd1e5a4c5cb1791f8; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userWorkspace"
    ADD CONSTRAINT "FK_a2da2ea7d6cd1e5a4c5cb1791f8" FOREIGN KEY ("userId") REFERENCES core."user"(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicateGroup FK_a41b6a06e3a7ded2204b0fc815d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicateGroup"
    ADD CONSTRAINT "FK_a41b6a06e3a7ded2204b0fc815d" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: logicFunction FK_a6ff4745db9bbe5a9616cfdfd5b; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."logicFunction"
    ADD CONSTRAINT "FK_a6ff4745db9bbe5a9616cfdfd5b" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: roleTarget FK_a86894bed7b7e1cc8b3f1d6186f; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "FK_a86894bed7b7e1cc8b3f1d6186f" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: workflowVersion FK_a94dcf82da41ca8e07cc6d912a9; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."workflowVersion"
    ADD CONSTRAINT "FK_a94dcf82da41ca8e07cc6d912a9" FOREIGN KEY ("coreWorkflowId") REFERENCES core.workflow(id) ON DELETE CASCADE;


--
-- Name: dpaAgreement FK_abba2f6707bd2bc18bbd52f3c3e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."dpaAgreement"
    ADD CONSTRAINT "FK_abba2f6707bd2bc18bbd52f3c3e" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: commandMenuItem FK_ad42dd64b117491a38120466d65; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."commandMenuItem"
    ADD CONSTRAINT "FK_ad42dd64b117491a38120466d65" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: twoFactorAuthenticationMethod FK_b0f44ffd7c794beb48cb1e1b1a9; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."twoFactorAuthenticationMethod"
    ADD CONSTRAINT "FK_b0f44ffd7c794beb48cb1e1b1a9" FOREIGN KEY ("userWorkspaceId") REFERENCES core."userWorkspace"(id) ON DELETE CASCADE;


--
-- Name: roleTarget FK_b1db027b64f44029389ace305ac; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "FK_b1db027b64f44029389ace305ac" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: indexFieldMetadata FK_b20192c432612eb710801dd5664; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."indexFieldMetadata"
    ADD CONSTRAINT "FK_b20192c432612eb710801dd5664" FOREIGN KEY ("indexMetadataId") REFERENCES core."indexMetadata"(id) ON DELETE CASCADE;


--
-- Name: permissionFlag FK_b26a9d39a88d0e72373c677c6c5; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."permissionFlag"
    ADD CONSTRAINT "FK_b26a9d39a88d0e72373c677c6c5" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_b2e02050a5faa58ed3e08624659; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_b2e02050a5faa58ed3e08624659" FOREIGN KEY ("userWorkspaceId") REFERENCES core."userWorkspace"(id) ON DELETE CASCADE;


--
-- Name: view FK_b3cc95732479f7a1337350c398f; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.view
    ADD CONSTRAINT "FK_b3cc95732479f7a1337350c398f" FOREIGN KEY ("kanbanAggregateOperationFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: viewFilter FK_b518bd61175e0963370e09ef15e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilter"
    ADD CONSTRAINT "FK_b518bd61175e0963370e09ef15e" FOREIGN KEY ("viewFilterGroupId") REFERENCES core."viewFilterGroup"(id) ON DELETE CASCADE;


--
-- Name: viewField FK_b560ea62a958deff0c6059caa45; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewField"
    ADD CONSTRAINT "FK_b560ea62a958deff0c6059caa45" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_b575e84d5ba7f183079c5c8c421; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_b575e84d5ba7f183079c5c8c421" FOREIGN KEY ("workspaceMemberFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE SET NULL;


--
-- Name: frontComponent FK_b5e4eea33659f066e865ab6afe0; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."frontComponent"
    ADD CONSTRAINT "FK_b5e4eea33659f066e865ab6afe0" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: mcpServer FK_b5eae743ed1d0658b26a73a79c6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."mcpServer"
    ADD CONSTRAINT "FK_b5eae743ed1d0658b26a73a79c6" FOREIGN KEY ("connectionProviderId") REFERENCES core."connectionProvider"(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_b617a57c424cc79e59b6560172d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_b617a57c424cc79e59b6560172d" FOREIGN KEY ("rowLevelPermissionRuleId") REFERENCES core."rowLevelPermissionRule"(id) ON DELETE CASCADE;


--
-- Name: twoFactorAuthenticationMethod FK_b8282d1e10fbb7856950f86c616; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."twoFactorAuthenticationMethod"
    ADD CONSTRAINT "FK_b8282d1e10fbb7856950f86c616" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: skill FK_b832ffda9048fae83e52fbe48a7; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.skill
    ADD CONSTRAINT "FK_b832ffda9048fae83e52fbe48a7" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: calendarChannel FK_bb5ebadf91b73c8050fb0a092fa; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."calendarChannel"
    ADD CONSTRAINT "FK_bb5ebadf91b73c8050fb0a092fa" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: fieldPermission FK_bbf16a91f5a10199e5b18c019ba; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldPermission"
    ADD CONSTRAINT "FK_bbf16a91f5a10199e5b18c019ba" FOREIGN KEY ("roleId") REFERENCES core.role(id) ON DELETE CASCADE;


--
-- Name: workspaceSSOIdentityProvider FK_bc8d8855198de1fbc32fba8df93; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."workspaceSSOIdentityProvider"
    ADD CONSTRAINT "FK_bc8d8855198de1fbc32fba8df93" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: fieldPermission FK_bd35d2494763b58ce85f13987fb; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldPermission"
    ADD CONSTRAINT "FK_bd35d2494763b58ce85f13987fb" FOREIGN KEY ("rowLevelPermissionRuleId") REFERENCES core."rowLevelPermissionRule"(id) ON DELETE CASCADE;


--
-- Name: indexFieldMetadata FK_be0950612a54b58c72bd62d629e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."indexFieldMetadata"
    ADD CONSTRAINT "FK_be0950612a54b58c72bd62d629e" FOREIGN KEY ("fieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: viewFilterGroup FK_bfc3498b964ef1bfc89b1f2bee3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilterGroup"
    ADD CONSTRAINT "FK_bfc3498b964ef1bfc89b1f2bee3" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: mcpServer FK_c0b38408081e8b6d0f661cd51fb; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."mcpServer"
    ADD CONSTRAINT "FK_c0b38408081e8b6d0f661cd51fb" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: keyValuePair FK_c137e3d8b3980901e114941daa2; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."keyValuePair"
    ADD CONSTRAINT "FK_c137e3d8b3980901e114941daa2" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: relatedRecordPath FK_c15bed1409e7a69ac2dfbbd6ea4; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."relatedRecordPath"
    ADD CONSTRAINT "FK_c15bed1409e7a69ac2dfbbd6ea4" FOREIGN KEY ("secondFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: agent FK_c4cb56621768a4a325dd772bbe1; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.agent
    ADD CONSTRAINT "FK_c4cb56621768a4a325dd772bbe1" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: pageLayoutWidget FK_c4dc95034f53a12601e623d9171; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayoutWidget"
    ADD CONSTRAINT "FK_c4dc95034f53a12601e623d9171" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: viewField FK_c5ab40cd4debb51d588752a4857; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewField"
    ADD CONSTRAINT "FK_c5ab40cd4debb51d588752a4857" FOREIGN KEY ("viewId") REFERENCES core.view(id) ON DELETE CASCADE;


--
-- Name: calendarChannel FK_c7bc368c97a18a072413d67cf45; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."calendarChannel"
    ADD CONSTRAINT "FK_c7bc368c97a18a072413d67cf45" FOREIGN KEY ("connectedAccountId") REFERENCES core."connectedAccount"(id) ON DELETE CASCADE;


--
-- Name: apiKey FK_c8b3efa54a29aa873043e72fb1d; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."apiKey"
    ADD CONSTRAINT "FK_c8b3efa54a29aa873043e72fb1d" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_c8c107480f88215057b8b1c9520; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_c8c107480f88215057b8b1c9520" FOREIGN KEY ("agentId") REFERENCES core.agent(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicateGroup FK_ca604fd5ee245bca9f32ed67b9b; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicateGroup"
    ADD CONSTRAINT "FK_ca604fd5ee245bca9f32ed67b9b" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: application FK_ca635da088fa8d5379ed268b55e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.application
    ADD CONSTRAINT "FK_ca635da088fa8d5379ed268b55e" FOREIGN KEY ("applicationRegistrationId") REFERENCES core."applicationRegistration"(id) ON DELETE SET NULL;


--
-- Name: rowLevelPermissionRule FK_d0282360068781f432cd0108cd2; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionRule"
    ADD CONSTRAINT "FK_d0282360068781f432cd0108cd2" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: applicationRegistrationVersion FK_d0a422e76f4d8289dd6c18c538c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistrationVersion"
    ADD CONSTRAINT "FK_d0a422e76f4d8289dd6c18c538c" FOREIGN KEY ("tarballFileId") REFERENCES core.file(id) ON DELETE SET NULL;


--
-- Name: view FK_d1fa625016e36ec6f79fb13e824; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.view
    ADD CONSTRAINT "FK_d1fa625016e36ec6f79fb13e824" FOREIGN KEY ("mainGroupByFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: role FK_d2532f520d84f8c22ee45681c5a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.role
    ADD CONSTRAINT "FK_d2532f520d84f8c22ee45681c5a" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: rolePermissionFlag FK_d47b1ebee75d98daa0c870c26e3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rolePermissionFlag"
    ADD CONSTRAINT "FK_d47b1ebee75d98daa0c870c26e3" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: viewFilter FK_d5651cf33fa56a47cd262a3fb2c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilter"
    ADD CONSTRAINT "FK_d5651cf33fa56a47cd262a3fb2c" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: applicationRegistration FK_d5aa70ce34f5b8e51e5b0deafc2; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."applicationRegistration"
    ADD CONSTRAINT "FK_d5aa70ce34f5b8e51e5b0deafc2" FOREIGN KEY ("createdByUserId") REFERENCES core."user"(id) ON DELETE SET NULL;


--
-- Name: fieldPermission FK_d5c47a26fe71648894d05da3d3a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldPermission"
    ADD CONSTRAINT "FK_d5c47a26fe71648894d05da3d3a" FOREIGN KEY ("fieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_d5ee96a9a03761f2c328a29523c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_d5ee96a9a03761f2c328a29523c" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: appToken FK_d6ae19a7aa2bbd4919053257772; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."appToken"
    ADD CONSTRAINT "FK_d6ae19a7aa2bbd4919053257772" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: viewField FK_d6f7c88260b1d4eaa8ad0f13c26; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewField"
    ADD CONSTRAINT "FK_d6f7c88260b1d4eaa8ad0f13c26" FOREIGN KEY ("viewFieldGroupId") REFERENCES core."viewFieldGroup"(id) ON DELETE SET NULL;


--
-- Name: objectMetadata FK_d82a05a204136c01388ea80bc7a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectMetadata"
    ADD CONSTRAINT "FK_d82a05a204136c01388ea80bc7a" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: connectionProviderOAuthClient FK_d8672886fdf748c49a458f5da8c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."connectionProviderOAuthClient"
    ADD CONSTRAINT "FK_d8672886fdf748c49a458f5da8c" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: indexFieldMetadata FK_d8cf7f15cf6466ac0e3b443b3d2; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."indexFieldMetadata"
    ADD CONSTRAINT "FK_d8cf7f15cf6466ac0e3b443b3d2" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: logicFunction FK_daed3cd4d8048fbe85646874615; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."logicFunction"
    ADD CONSTRAINT "FK_daed3cd4d8048fbe85646874615" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: fieldMetadataOption FK_db1306d4c94604c52d5db100668; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadataOption"
    ADD CONSTRAINT "FK_db1306d4c94604c52d5db100668" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: viewFilter FK_dbe259395cbd9a54c1c17d12b0b; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilter"
    ADD CONSTRAINT "FK_dbe259395cbd9a54c1c17d12b0b" FOREIGN KEY ("relationTargetFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: fieldPermission FK_dc8e552397f5e44d175fedf752a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldPermission"
    ADD CONSTRAINT "FK_dc8e552397f5e44d175fedf752a" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: viewFilterGroup FK_dce74ab06fa7a2effcbf1b98dff; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewFilterGroup"
    ADD CONSTRAINT "FK_dce74ab06fa7a2effcbf1b98dff" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: pageLayout FK_dd63ca42614bacf58971aabdcbb; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayout"
    ADD CONSTRAINT "FK_dd63ca42614bacf58971aabdcbb" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: fieldMetadata FK_de2a09b9e3e690440480d2dee26; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."fieldMetadata"
    ADD CONSTRAINT "FK_de2a09b9e3e690440480d2dee26" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: file FK_de468b3d8dcf7e94f7074220929; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.file
    ADD CONSTRAINT "FK_de468b3d8dcf7e94f7074220929" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: agentChatChannelRole FK_e01d14cd9ae854f33c70a6385da; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."agentChatChannelRole"
    ADD CONSTRAINT "FK_e01d14cd9ae854f33c70a6385da" FOREIGN KEY ("roleId") REFERENCES core.role(id) ON DELETE CASCADE;


--
-- Name: dataSource FK_e1914827ee8b22fba4254578311; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."dataSource"
    ADD CONSTRAINT "FK_e1914827ee8b22fba4254578311" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: view FK_e1d69dd7402cd7df3b03ce11311; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.view
    ADD CONSTRAINT "FK_e1d69dd7402cd7df3b03ce11311" FOREIGN KEY ("calendarEndFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE SET NULL;


--
-- Name: keyValuePair FK_e31d245e30cd82307e5416450fc; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."keyValuePair"
    ADD CONSTRAINT "FK_e31d245e30cd82307e5416450fc" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: roleTarget FK_e3c4f91f74bc6092c81b63edf39; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."roleTarget"
    ADD CONSTRAINT "FK_e3c4f91f74bc6092c81b63edf39" FOREIGN KEY ("webhookId") REFERENCES core.webhook(id) ON DELETE CASCADE;


--
-- Name: webPushSubscription FK_e5fb7bca454ec469ad0952cf8b4; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."webPushSubscription"
    ADD CONSTRAINT "FK_e5fb7bca454ec469ad0952cf8b4" FOREIGN KEY ("userSessionId") REFERENCES core."userSession"(id) ON DELETE CASCADE;


--
-- Name: validationRule FK_e640b1e4b64ac470c2fa0f29830; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."validationRule"
    ADD CONSTRAINT "FK_e640b1e4b64ac470c2fa0f29830" FOREIGN KEY ("errorFieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE SET NULL;


--
-- Name: webhook FK_e755f49a9ef74b36e27932f7a6c; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.webhook
    ADD CONSTRAINT "FK_e755f49a9ef74b36e27932f7a6c" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: messageFolder FK_e7fb85af997d06d8f7cc7512801; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."messageFolder"
    ADD CONSTRAINT "FK_e7fb85af997d06d8f7cc7512801" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicate FK_eadcbbb92b6d58c6785a780b5b7; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicate"
    ADD CONSTRAINT "FK_eadcbbb92b6d58c6785a780b5b7" FOREIGN KEY ("fieldMetadataId") REFERENCES core."fieldMetadata"(id) ON DELETE CASCADE;


--
-- Name: objectPermission FK_edcd87df18d3284141757bf6e16; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectPermission"
    ADD CONSTRAINT "FK_edcd87df18d3284141757bf6e16" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: userApplicationVariableValue FK_ef9ee4ba7549728c956374e77eb; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."userApplicationVariableValue"
    ADD CONSTRAINT "FK_ef9ee4ba7549728c956374e77eb" FOREIGN KEY ("applicationVariableId", scope) REFERENCES core."applicationVariable"(id, scope) ON DELETE CASCADE;


--
-- Name: objectPermission FK_efbcf3528718de2b5c45c0a8a83; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectPermission"
    ADD CONSTRAINT "FK_efbcf3528718de2b5c45c0a8a83" FOREIGN KEY ("objectMetadataId") REFERENCES core."objectMetadata"(id) ON DELETE CASCADE;


--
-- Name: objectPermission FK_f2ecee1066fd43800dbc85f87e4; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."objectPermission"
    ADD CONSTRAINT "FK_f2ecee1066fd43800dbc85f87e4" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: validationRule FK_f664e9abe4db4c6104554529d94; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."validationRule"
    ADD CONSTRAINT "FK_f664e9abe4db4c6104554529d94" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: pageLayoutWidget FK_fb84d310b4cfe5916ced6fc3e2a; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."pageLayoutWidget"
    ADD CONSTRAINT "FK_fb84d310b4cfe5916ced6fc3e2a" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionRule FK_fb9ca25039bd4009d2af1ad721f; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionRule"
    ADD CONSTRAINT "FK_fb9ca25039bd4009d2af1ad721f" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: webPushSubscription FK_fbb92c160a36a1ef200bdc882d3; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."webPushSubscription"
    ADD CONSTRAINT "FK_fbb92c160a36a1ef200bdc882d3" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: workflow FK_fbce9a986a577698821a7e301b6; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.workflow
    ADD CONSTRAINT "FK_fbce9a986a577698821a7e301b6" FOREIGN KEY ("workspaceId") REFERENCES core.workspace(id) ON DELETE CASCADE;


--
-- Name: rowLevelPermissionPredicateGroup FK_fee64b361fd3e8eeee275125d0e; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."rowLevelPermissionPredicateGroup"
    ADD CONSTRAINT "FK_fee64b361fd3e8eeee275125d0e" FOREIGN KEY ("rowLevelPermissionRuleId") REFERENCES core."rowLevelPermissionRule"(id) ON DELETE CASCADE;


--
-- Name: file FK_feffd2addf9467be6d7cd51db76; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core.file
    ADD CONSTRAINT "FK_feffd2addf9467be6d7cd51db76" FOREIGN KEY ("applicationRegistrationId") REFERENCES core."applicationRegistration"(id) ON DELETE CASCADE;


--
-- Name: navigationMenuItem FK_ff45ab0f2035e907848cd47b1f4; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."navigationMenuItem"
    ADD CONSTRAINT "FK_ff45ab0f2035e907848cd47b1f4" FOREIGN KEY ("parentId") REFERENCES core."navigationMenuItem"(id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;


--
-- Name: viewSort FK_ff8cbebe1704954120df82bf393; Type: FK CONSTRAINT; Schema: core; Owner: -
--

ALTER TABLE ONLY core."viewSort"
    ADD CONSTRAINT "FK_ff8cbebe1704954120df82bf393" FOREIGN KEY ("applicationId") REFERENCES core.application(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict 6W0R2gj9jRAjN853g1GIhxmUoG2xs8igFBTxhcqraV00JKi70yKCCjefkFn6yQR

