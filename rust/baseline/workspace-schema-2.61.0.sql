--
-- PostgreSQL database dump
--

\restrict oBWiH0mrTa4WiBcUuNf82QTO3l2MXJiewyaCWDdlVwEFyWeV2IlDZ80tz9Cyfr8

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
-- Name: workspace_6n57znb4w89f8r7xudebmf0; Type: SCHEMA; Schema: -; Owner: -
--

CREATE SCHEMA workspace_6n57znb4w89f8r7xudebmf0;


--
-- Name: accessGroup_type_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."accessGroup_type_enum" AS ENUM (
    'ALL_RECORDS',
    'RLS_RULE',
    'RLS_MEMBER_VALUE',
    'RLS_LITERAL_VALUE'
);


--
-- Name: agentChatThread_conversationType_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."agentChatThread_conversationType_enum" AS ENUM (
    'STANDARD',
    'DIRECT_MESSAGE'
);


--
-- Name: agentTurn_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."agentTurn_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: attachment_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."attachment_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: attachment_fileCategory_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."attachment_fileCategory_enum" AS ENUM (
    'ARCHIVE',
    'AUDIO',
    'IMAGE',
    'PRESENTATION',
    'SPREADSHEET',
    'TEXT_DOCUMENT',
    'VIDEO',
    'OTHER'
);


--
-- Name: attachment_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."attachment_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: blocklist_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."blocklist_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: blocklist_scope_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0.blocklist_scope_enum AS ENUM (
    'WORKSPACE_MEMBER',
    'WORKSPACE'
);


--
-- Name: blocklist_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."blocklist_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: calendarChannelEventAssociation_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: calendarChannelEventAssociation_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: calendarEventParticipant_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: calendarEventParticipant_responseStatus_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant_responseStatus_enum" AS ENUM (
    'NEEDS_ACTION',
    'DECLINED',
    'TENTATIVE',
    'ACCEPTED'
);


--
-- Name: calendarEventParticipant_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: calendarEventTarget_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: calendarEventTarget_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: calendarEvent_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."calendarEvent_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: calendarEvent_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."calendarEvent_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: callRecording_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."callRecording_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: callRecording_recordingRequestStatus_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."callRecording_recordingRequestStatus_enum" AS ENUM (
    'REQUESTED',
    'CANCELED'
);


--
-- Name: callRecording_status_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."callRecording_status_enum" AS ENUM (
    'SCHEDULED',
    'JOINING',
    'RECORDING',
    'PROCESSING',
    'COMPLETED',
    'FAILED',
    'NOT_RECORDED'
);


--
-- Name: callRecording_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."callRecording_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: company_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."company_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: company_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."company_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: dashboard_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."dashboard_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: dashboard_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."dashboard_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageCampaign_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageCampaign_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageCampaign_status_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageCampaign_status_enum" AS ENUM (
    'DRAFT',
    'SCHEDULED',
    'SENDING',
    'SENT',
    'SENT_WITH_ERRORS',
    'CANCELED'
);


--
-- Name: messageCampaign_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageCampaign_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageChannelMessageAssociationMessageFolder_createdBySource_e; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder_createdBySource_e" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageChannelMessageAssociationMessageFolder_updatedBySource_e; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder_updatedBySource_e" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageChannelMessageAssociation_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageChannelMessageAssociation_direction_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation_direction_enum" AS ENUM (
    'INCOMING',
    'OUTGOING'
);


--
-- Name: messageChannelMessageAssociation_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageListMember_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageListMember_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageListMember_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageListMember_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageList_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageList_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageList_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageList_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageParticipant_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageParticipant_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageParticipant_role_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageParticipant_role_enum" AS ENUM (
    'FROM',
    'TO',
    'CC',
    'BCC',
    'REPLY_TO'
);


--
-- Name: messageParticipant_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageParticipant_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageThreadTarget_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageThreadTarget_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageThread_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageThread_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: messageThread_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."messageThread_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: message_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."message_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: message_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."message_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: noteTarget_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."noteTarget_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: noteTarget_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."noteTarget_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: note_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."note_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: note_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."note_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: opportunity_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."opportunity_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: opportunity_stage_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0.opportunity_stage_enum AS ENUM (
    'NEW',
    'SCREENING',
    'MEETING',
    'PROPOSAL',
    'CUSTOMER'
);


--
-- Name: opportunity_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."opportunity_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: person_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."person_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: person_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."person_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: recordShare_accessLevel_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."recordShare_accessLevel_enum" AS ENUM (
    'READ',
    'READ_WRITE',
    'FULL',
    'NONE'
);


--
-- Name: recordShare_principalType_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."recordShare_principalType_enum" AS ENUM (
    'EVERYONE',
    'WORKSPACE_MEMBER',
    'ROLE'
);


--
-- Name: recordShare_rowCause_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."recordShare_rowCause_enum" AS ENUM (
    'OWNER',
    'MANUAL',
    'RULE',
    'APPLICATION'
);


--
-- Name: shortLink_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."shortLink_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: shortLink_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."shortLink_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: taskTarget_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."taskTarget_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: taskTarget_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."taskTarget_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: task_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."task_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: task_status_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0.task_status_enum AS ENUM (
    'TODO',
    'IN_PROGRESS',
    'DONE'
);


--
-- Name: task_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."task_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: timelineActivity_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."timelineActivity_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: timelineActivity_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."timelineActivity_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: workflowRun_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."workflowRun_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: workflowRun_status_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."workflowRun_status_enum" AS ENUM (
    'NOT_STARTED',
    'RUNNING',
    'COMPLETED',
    'FAILED',
    'ENQUEUED',
    'STOPPING',
    'STOPPED'
);


--
-- Name: workflowRun_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."workflowRun_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: workspaceMember_createdBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_createdBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


--
-- Name: workspaceMember_dateFormat_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_dateFormat_enum" AS ENUM (
    'SYSTEM',
    'MONTH_FIRST',
    'DAY_FIRST',
    'YEAR_FIRST'
);


--
-- Name: workspaceMember_numberFormat_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_numberFormat_enum" AS ENUM (
    'SYSTEM',
    'COMMAS_AND_DOT',
    'SPACES_AND_COMMA',
    'DOTS_AND_COMMA',
    'APOSTROPHE_AND_DOT'
);


--
-- Name: workspaceMember_timeFormat_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_timeFormat_enum" AS ENUM (
    'SYSTEM',
    'HOUR_24',
    'HOUR_12'
);


--
-- Name: workspaceMember_updatedBySource_enum; Type: TYPE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TYPE workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_updatedBySource_enum" AS ENUM (
    'EMAIL',
    'CALENDAR',
    'WORKFLOW',
    'AGENT',
    'API',
    'IMPORT',
    'MANUAL',
    'SYSTEM',
    'WEBHOOK',
    'APPLICATION'
);


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: accessGroup; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."accessGroup" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "objectMetadataId" uuid,
    type workspace_6n57znb4w89f8r7xudebmf0."accessGroup_type_enum" NOT NULL,
    "ruleShapeHash" character varying NOT NULL,
    "unreadableFieldIds" uuid[] DEFAULT '{}'::uuid[] NOT NULL,
    "memberFieldValues" jsonb,
    value character varying,
    "groupHash" text NOT NULL,
    "compactId" integer NOT NULL,
    CONSTRAINT "CHK_ACCESS_GROUP_TYPE_COLUMNS" CHECK ((((type = ANY (ARRAY['ALL_RECORDS'::workspace_6n57znb4w89f8r7xudebmf0."accessGroup_type_enum", 'RLS_RULE'::workspace_6n57znb4w89f8r7xudebmf0."accessGroup_type_enum"])) AND ("memberFieldValues" IS NULL) AND (value IS NULL)) OR ((type = 'RLS_MEMBER_VALUE'::workspace_6n57znb4w89f8r7xudebmf0."accessGroup_type_enum") AND ("memberFieldValues" IS NOT NULL) AND (value IS NULL)) OR ((type = 'RLS_LITERAL_VALUE'::workspace_6n57znb4w89f8r7xudebmf0."accessGroup_type_enum") AND ("memberFieldValues" IS NULL) AND (value IS NOT NULL))))
);


--
-- Name: accessGroup_compactId_seq; Type: SEQUENCE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE workspace_6n57znb4w89f8r7xudebmf0."accessGroup" ALTER COLUMN "compactId" ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME workspace_6n57znb4w89f8r7xudebmf0."accessGroup_compactId_seq"
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: agentChatThread; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."agentChatThread" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    title text,
    "conversationType" workspace_6n57znb4w89f8r7xudebmf0."agentChatThread_conversationType_enum" DEFAULT 'STANDARD'::workspace_6n57znb4w89f8r7xudebmf0."agentChatThread_conversationType_enum" NOT NULL,
    "directMessageFirstWorkspaceMemberId" uuid,
    "directMessageSecondWorkspaceMemberId" uuid,
    "isAssistantParticipant" boolean DEFAULT true NOT NULL,
    "totalInputTokens" double precision DEFAULT '0'::double precision NOT NULL,
    "totalOutputTokens" double precision DEFAULT '0'::double precision NOT NULL,
    "contextWindowTokens" double precision,
    "conversationSize" double precision DEFAULT '0'::double precision NOT NULL,
    "totalInputCredits" numeric DEFAULT '0'::numeric NOT NULL,
    "totalOutputCredits" numeric DEFAULT '0'::numeric NOT NULL,
    "totalCacheReadTokens" numeric DEFAULT '0'::numeric NOT NULL,
    "totalCacheCreationTokens" numeric DEFAULT '0'::numeric NOT NULL,
    "activeStreamId" text,
    "pendingQuestionMessageId" uuid,
    "deletedAt" timestamp with time zone,
    "archivedAt" timestamp with time zone,
    "lastActivityAt" timestamp with time zone,
    "lastMessageText" text,
    "lastMessageSenderWorkspaceMemberId" uuid,
    "writerWorkspaceMemberIds" text[],
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "channelId" uuid,
    "channelArchivedAt" timestamp with time zone,
    "channelSnoozedUntil" timestamp with time zone,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "workspaceMemberId" uuid,
    "assigneeId" uuid
);


--
-- Name: agentChatThreadParticipant; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadParticipant" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "lastReadAt" timestamp with time zone,
    "archivedAt" timestamp with time zone,
    "snoozedUntil" timestamp with time zone,
    "isSubscribed" boolean DEFAULT true NOT NULL,
    "lastMentionedAt" timestamp with time zone,
    "mentionsResolvedAt" timestamp with time zone,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "threadId" uuid,
    "workspaceMemberId" uuid
);


--
-- Name: agentChatThreadTarget; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "threadId" uuid,
    "targetPersonId" uuid,
    "targetCompanyId" uuid,
    "targetOpportunityId" uuid
);


--
-- Name: agentMessage; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."agentMessage" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "agentId" uuid,
    "senderApplicationId" uuid,
    role text NOT NULL,
    status text DEFAULT 'sent'::text NOT NULL,
    "processedAt" timestamp with time zone,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "senderWorkspaceMemberId" uuid,
    "threadId" uuid,
    "turnId" uuid
);


--
-- Name: agentMessagePart; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."agentMessagePart" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "orderIndex" double precision NOT NULL,
    type text NOT NULL,
    "textContent" text,
    "reasoningContent" text,
    "toolName" text,
    "toolCallId" text,
    "toolInput" jsonb,
    "toolOutput" jsonb,
    state text,
    "providerExecuted" boolean,
    "errorMessage" text,
    "errorDetails" jsonb,
    "sourceUrlSourceId" text,
    "sourceUrlUrl" text,
    "sourceUrlTitle" text,
    "sourceDocumentSourceId" text,
    "sourceDocumentMediaType" text,
    "sourceDocumentTitle" text,
    "sourceDocumentFilename" text,
    "fileFilename" text,
    "fileId" uuid,
    "providerMetadata" jsonb,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "messageId" uuid
);


--
-- Name: agentMessageReaction; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."agentMessageReaction" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "agentId" uuid,
    emoji text NOT NULL,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "messageId" uuid,
    "workspaceMemberId" uuid
);


--
-- Name: agentTurn; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."agentTurn" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "agentId" uuid,
    status text DEFAULT 'completed'::text NOT NULL,
    error jsonb,
    "startedAt" timestamp with time zone,
    "endedAt" timestamp with time zone,
    "modelId" text,
    "inputTokens" double precision,
    "outputTokens" double precision,
    "cacheReadTokens" numeric,
    "cacheCreationTokens" numeric,
    "inputCredits" numeric,
    "outputCredits" numeric,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."agentTurn_createdBySource_enum" DEFAULT 'SYSTEM'::workspace_6n57znb4w89f8r7xudebmf0."agentTurn_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "threadId" uuid
);


--
-- Name: attachment; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0.attachment (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    name text,
    file jsonb,
    "fullPath" text,
    "fileCategory" workspace_6n57znb4w89f8r7xudebmf0."attachment_fileCategory_enum" DEFAULT 'OTHER'::workspace_6n57znb4w89f8r7xudebmf0."attachment_fileCategory_enum" NOT NULL,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."attachment_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."attachment_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."attachment_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."attachment_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(name), ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "targetAgentChatThreadId" uuid,
    "targetTaskId" uuid,
    "targetNoteId" uuid,
    "targetPersonId" uuid,
    "targetCompanyId" uuid,
    "targetOpportunityId" uuid,
    "targetDashboardId" uuid,
    "targetMessageId" uuid
);


--
-- Name: blocklist; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0.blocklist (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."blocklist_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."blocklist_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."blocklist_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."blocklist_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(handle), ''::text))) STORED,
    handle text,
    scope workspace_6n57znb4w89f8r7xudebmf0.blocklist_scope_enum DEFAULT 'WORKSPACE_MEMBER'::workspace_6n57znb4w89f8r7xudebmf0.blocklist_scope_enum NOT NULL,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "workspaceMemberId" uuid
);


--
-- Name: calendarChannelEventAssociation; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable("eventExternalId"), ''::text))) STORED,
    "eventExternalId" text,
    "recurringEventExternalId" text,
    "calendarChannelId" uuid NOT NULL,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "calendarEventId" uuid
);


--
-- Name: calendarEvent; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."calendarEvent" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."calendarEvent_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."calendarEvent_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."calendarEvent_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."calendarEvent_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(title), ''::text))) STORED,
    title text,
    "isCanceled" boolean DEFAULT false NOT NULL,
    "isFullDay" boolean DEFAULT false NOT NULL,
    "startsAt" timestamp with time zone,
    "endsAt" timestamp with time zone,
    "externalCreatedAt" timestamp with time zone,
    "externalUpdatedAt" timestamp with time zone,
    description text,
    location text,
    "iCalUid" text,
    "conferenceSolution" text,
    "conferenceLinkPrimaryLinkLabel" text,
    "conferenceLinkPrimaryLinkUrl" text,
    "conferenceLinkSecondaryLinks" jsonb,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: calendarEventParticipant; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(handle), ''::text))) STORED,
    handle text,
    "displayName" text,
    "isOrganizer" boolean DEFAULT false NOT NULL,
    "responseStatus" workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant_responseStatus_enum" DEFAULT 'NEEDS_ACTION'::workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant_responseStatus_enum" NOT NULL,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "calendarEventId" uuid,
    "personId" uuid,
    "workspaceMemberId" uuid
);


--
-- Name: calendarEventTarget; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE((id)::text, ''::text))) STORED,
    "isAutomaticallyAssigned" boolean DEFAULT false NOT NULL,
    "isManuallyAssigned" boolean DEFAULT true NOT NULL,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "calendarEventId" uuid,
    "targetPersonId" uuid,
    "targetCompanyId" uuid,
    "targetOpportunityId" uuid
);


--
-- Name: callRecording; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."callRecording" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    title text,
    status workspace_6n57znb4w89f8r7xudebmf0."callRecording_status_enum" DEFAULT 'SCHEDULED'::workspace_6n57znb4w89f8r7xudebmf0."callRecording_status_enum" NOT NULL,
    "recordingRequestStatus" workspace_6n57znb4w89f8r7xudebmf0."callRecording_recordingRequestStatus_enum" DEFAULT 'REQUESTED'::workspace_6n57znb4w89f8r7xudebmf0."callRecording_recordingRequestStatus_enum" NOT NULL,
    "applicationId" uuid,
    "externalBotId" text,
    "externalRecordingId" text,
    "startedAt" timestamp with time zone,
    "endedAt" timestamp with time zone,
    video jsonb,
    audio jsonb,
    transcript jsonb,
    "summaryBlocknote" text,
    "summaryMarkdown" text,
    "summaryTiptap" text,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."callRecording_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."callRecording_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."callRecording_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."callRecording_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(title), ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "calendarEventId" uuid
);


--
-- Name: campaignDelivery; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."campaignDelivery" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "campaignId" uuid NOT NULL,
    "personId" uuid NOT NULL,
    "recipientEmail" text NOT NULL,
    state text DEFAULT 'QUEUED'::text NOT NULL,
    "skipReason" text,
    "failureReason" text,
    "claimToken" uuid,
    "claimExpiresAt" timestamp with time zone,
    "providerMessageId" text,
    "sentAt" timestamp with time zone,
    "deliveredAt" timestamp with time zone,
    "bouncedAt" timestamp with time zone,
    "complainedAt" timestamp with time zone,
    "rejectedAt" timestamp with time zone,
    "renderingFailedAt" timestamp with time zone,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: company; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0.company (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    name text,
    "domainNamePrimaryLinkLabel" text,
    "domainNamePrimaryLinkUrl" text,
    "domainNameSecondaryLinks" jsonb,
    "addressAddressStreet1" text,
    "addressAddressStreet2" text,
    "addressAddressCity" text,
    "addressAddressPostcode" text,
    "addressAddressState" text,
    "addressAddressCountry" text,
    "addressAddressLat" numeric,
    "addressAddressLng" numeric,
    "linkedinLinkPrimaryLinkLabel" text,
    "linkedinLinkPrimaryLinkUrl" text,
    "linkedinLinkSecondaryLinks" jsonb,
    "annualRevenueAmountMicros" numeric,
    "annualRevenueCurrencyCode" text,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."company_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."company_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."company_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."company_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, ((((((COALESCE(public.unaccent_immutable(name), ''::text) || ' '::text) || COALESCE(public.unaccent_immutable("domainNamePrimaryLinkLabel"), ''::text)) || ' '::text) || COALESCE(public.unaccent_immutable("domainNamePrimaryLinkUrl"), ''::text)) || ' '::text) || COALESCE(public.unaccent_immutable(translate(regexp_replace(("domainNameSecondaryLinks")::text, '"(label|url)"\s*:\s*'::text, ''::text, 'g'::text), '[]{}",:'::text, '        '::text)), ''::text)))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "accountOwnerId" uuid
);


--
-- Name: dashboard; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0.dashboard (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    title text,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "pageLayoutId" uuid,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."dashboard_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."dashboard_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."dashboard_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."dashboard_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(title), ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: message; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0.message (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."message_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."message_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."message_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."message_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(subject), ''::text))) STORED,
    "headerMessageId" text,
    subject text,
    text text,
    "receivedAt" timestamp with time zone,
    "isDraft" boolean DEFAULT false NOT NULL,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "messageCampaignId" uuid,
    "messageThreadId" uuid
);


--
-- Name: messageCampaign; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."messageCampaign" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."messageCampaign_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageCampaign_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."messageCampaign_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageCampaign_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, ((COALESCE(public.unaccent_immutable(name), ''::text) || ' '::text) || COALESCE(public.unaccent_immutable(subject), ''::text)))) STORED,
    name text DEFAULT ''::text NOT NULL,
    subject text,
    "bodyTemplate" text,
    "fromAddressPrimaryEmail" text,
    "fromAddressAdditionalEmails" jsonb,
    status workspace_6n57znb4w89f8r7xudebmf0."messageCampaign_status_enum" DEFAULT 'DRAFT'::workspace_6n57znb4w89f8r7xudebmf0."messageCampaign_status_enum" NOT NULL,
    "scheduledAt" timestamp with time zone,
    "sentAt" timestamp with time zone,
    "sentCount" double precision DEFAULT '0'::double precision NOT NULL,
    "deliveredCount" double precision DEFAULT '0'::double precision NOT NULL,
    "failedCount" double precision DEFAULT '0'::double precision NOT NULL,
    "bouncedCount" double precision DEFAULT '0'::double precision NOT NULL,
    "complainedCount" double precision DEFAULT '0'::double precision NOT NULL,
    "skippedCount" double precision DEFAULT '0'::double precision NOT NULL,
    "uniqueClickCount" double precision DEFAULT '0'::double precision NOT NULL,
    "clickRate" double precision DEFAULT '0'::double precision NOT NULL,
    "uniqueOpenCount" double precision DEFAULT '0'::double precision NOT NULL,
    "openRate" double precision DEFAULT '0'::double precision NOT NULL,
    "unsubscribeTopicId" uuid,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "listId" uuid
);


--
-- Name: messageChannelMessageAssociation; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable("messageExternalId"), ''::text))) STORED,
    "messageExternalId" text,
    "messageThreadExternalId" text,
    direction workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation_direction_enum" DEFAULT 'INCOMING'::workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation_direction_enum" NOT NULL,
    "messageChannelId" uuid,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "messageId" uuid,
    "messageThreadId" uuid
);


--
-- Name: messageChannelMessageAssociationMessageFolder; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder_createdBySource_e" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder_createdBySource_e",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder_updatedBySource_e" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder_updatedBySource_e",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, NULL::text)) STORED,
    "messageFolderId" uuid NOT NULL,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "messageChannelMessageAssociationId" uuid
);


--
-- Name: messageList; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."messageList" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."messageList_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageList_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."messageList_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageList_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    name text,
    description text,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(name), ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: messageListMember; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."messageListMember" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."messageListMember_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageListMember_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."messageListMember_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageListMember_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE((id)::text, ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "listId" uuid,
    "personId" uuid
);


--
-- Name: messageParticipant; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."messageParticipant" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."messageParticipant_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageParticipant_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."messageParticipant_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageParticipant_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(handle), ''::text))) STORED,
    role workspace_6n57znb4w89f8r7xudebmf0."messageParticipant_role_enum" DEFAULT 'FROM'::workspace_6n57znb4w89f8r7xudebmf0."messageParticipant_role_enum" NOT NULL,
    handle text,
    "displayName" text,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "messageCampaignId" uuid,
    "messageId" uuid,
    "personId" uuid,
    "workspaceMemberId" uuid
);


--
-- Name: messageSuppression; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."messageSuppression" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "emailAddress" text NOT NULL,
    reason text NOT NULL,
    source text NOT NULL,
    "providerEventId" text,
    "unsubscribeTopicId" uuid,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: messageThread; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."messageThread" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."messageThread_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageThread_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."messageThread_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageThread_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(subject), ''::text))) STORED,
    subject text,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: messageThreadTarget; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE((id)::text, ''::text))) STORED,
    "isAutomaticallyAssigned" boolean DEFAULT false NOT NULL,
    "isManuallyAssigned" boolean DEFAULT true NOT NULL,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "targetCompanyId" uuid,
    "messageThreadId" uuid,
    "targetPersonId" uuid,
    "targetOpportunityId" uuid
);


--
-- Name: note; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0.note (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    title text,
    "bodyV2Blocknote" text,
    "bodyV2Markdown" text,
    "bodyV2Tiptap" text,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."note_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."note_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."note_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."note_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, ((COALESCE(public.unaccent_immutable(title), ''::text) || ' '::text) || COALESCE(public.unaccent_immutable("bodyV2Markdown"), ''::text)))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: noteTarget; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."noteTarget" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."noteTarget_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."noteTarget_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."noteTarget_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."noteTarget_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE((id)::text, ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "targetCompanyId" uuid,
    "noteId" uuid,
    "targetPersonId" uuid,
    "targetOpportunityId" uuid
);


--
-- Name: opportunity; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0.opportunity (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    name text,
    "amountAmountMicros" numeric,
    "amountCurrencyCode" text,
    "closeDate" timestamp with time zone,
    stage workspace_6n57znb4w89f8r7xudebmf0.opportunity_stage_enum DEFAULT 'NEW'::workspace_6n57znb4w89f8r7xudebmf0.opportunity_stage_enum NOT NULL,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."opportunity_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."opportunity_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."opportunity_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."opportunity_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(name), ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "companyId" uuid,
    "pointOfContactId" uuid,
    "ownerId" uuid
);


--
-- Name: person; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0.person (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "nameFirstName" text,
    "nameLastName" text,
    "emailsPrimaryEmail" text,
    "emailsAdditionalEmails" jsonb,
    "linkedinLinkPrimaryLinkLabel" text,
    "linkedinLinkPrimaryLinkUrl" text,
    "linkedinLinkSecondaryLinks" jsonb,
    "jobTitle" text,
    "phonesPrimaryPhoneNumber" text,
    "phonesPrimaryPhoneCountryCode" text,
    "phonesPrimaryPhoneCallingCode" text,
    "phonesAdditionalPhones" jsonb,
    "avatarUrl" text,
    "avatarFile" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."person_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."person_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."person_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."person_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, ((((((((((((((((((((((((COALESCE(public.unaccent_immutable("nameFirstName"), ''::text) || ' '::text) || COALESCE(public.unaccent_immutable("nameLastName"), ''::text)) || ' '::text) || COALESCE(public.unaccent_immutable("emailsPrimaryEmail"), ''::text)) || ' '::text) || COALESCE(public.unaccent_immutable(split_part("emailsPrimaryEmail", '@'::text, 2)), ''::text)) || ' '::text) || COALESCE(public.unaccent_immutable(translate(("emailsAdditionalEmails")::text, '[]",'::text, '    '::text)), ''::text)) || ' '::text) || COALESCE(public.unaccent_immutable(translate(replace(("emailsAdditionalEmails")::text, '@'::text, ' '::text), '[]",'::text, '    '::text)), ''::text)) || ' '::text) || COALESCE("phonesPrimaryPhoneNumber", ''::text)) || ' '::text) || COALESCE("phonesPrimaryPhoneCallingCode", ''::text)) || ' '::text) || COALESCE(("phonesPrimaryPhoneCallingCode" || "phonesPrimaryPhoneNumber"), ''::text)) || ' '::text) || COALESCE((replace("phonesPrimaryPhoneCallingCode", '+'::text, ''::text) || "phonesPrimaryPhoneNumber"), ''::text)) || ' '::text) || COALESCE(('0'::text || "phonesPrimaryPhoneNumber"), ''::text)) || ' '::text) || COALESCE(translate(regexp_replace(("phonesAdditionalPhones")::text, '"(number|countryCode|callingCode)"\s*:\s*'::text, ''::text, 'g'::text), '[]{}",:'::text, '        '::text), ''::text)) || ' '::text) || COALESCE(public.unaccent_immutable("jobTitle"), ''::text)))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "companyId" uuid
);


--
-- Name: recordShare; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."recordShare" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "recordId" uuid NOT NULL,
    "objectMetadataId" uuid NOT NULL,
    "principalId" uuid NOT NULL,
    "principalType" workspace_6n57znb4w89f8r7xudebmf0."recordShare_principalType_enum" NOT NULL,
    "accessLevel" workspace_6n57znb4w89f8r7xudebmf0."recordShare_accessLevel_enum" DEFAULT 'READ'::workspace_6n57znb4w89f8r7xudebmf0."recordShare_accessLevel_enum" NOT NULL,
    "rowCause" workspace_6n57znb4w89f8r7xudebmf0."recordShare_rowCause_enum" NOT NULL,
    "sourceId" uuid NOT NULL
);


--
-- Name: shortLink; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."shortLink" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."shortLink_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."shortLink_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."shortLink_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."shortLink_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, NULL::text)) STORED,
    "authoredTemplateUrl" text NOT NULL,
    "resolvedDestinationUrl" text NOT NULL,
    "templateAndResolvedUrlHash" text NOT NULL,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: task; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0.task (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    title text,
    "bodyV2Blocknote" text,
    "bodyV2Markdown" text,
    "bodyV2Tiptap" text,
    "dueAt" timestamp with time zone,
    status workspace_6n57znb4w89f8r7xudebmf0.task_status_enum DEFAULT 'TODO'::workspace_6n57znb4w89f8r7xudebmf0.task_status_enum,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."task_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."task_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."task_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."task_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, ((COALESCE(public.unaccent_immutable(title), ''::text) || ' '::text) || COALESCE(public.unaccent_immutable("bodyV2Markdown"), ''::text)))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "assigneeId" uuid
);


--
-- Name: taskTarget; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."taskTarget" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."taskTarget_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."taskTarget_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."taskTarget_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."taskTarget_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE((id)::text, ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "targetCompanyId" uuid,
    "targetOpportunityId" uuid,
    "targetPersonId" uuid,
    "taskId" uuid
);


--
-- Name: timelineActivity; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."timelineActivity_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."timelineActivity_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."timelineActivity_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."timelineActivity_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "happensAt" timestamp with time zone DEFAULT now() NOT NULL,
    "timelineActivityTypeId" uuid,
    "timelineActivityTypeSnapshot" jsonb,
    properties jsonb,
    "linkedRecordCachedName" text,
    "linkedRecordId" uuid,
    "linkedObjectMetadataId" uuid,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable("linkedRecordCachedName"), ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL,
    "targetCompanyId" uuid,
    "targetDashboardId" uuid,
    "targetMessageCampaignId" uuid,
    "targetMessageListId" uuid,
    "targetNoteId" uuid,
    "targetOpportunityId" uuid,
    "targetPersonId" uuid,
    "targetTaskId" uuid,
    "workspaceMemberId" uuid,
    "targetWorkflowRunId" uuid
);


--
-- Name: workflowRun; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."workflowRun" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    name text,
    "enqueuedAt" timestamp with time zone,
    "startedAt" timestamp with time zone,
    "endedAt" timestamp with time zone,
    status workspace_6n57znb4w89f8r7xudebmf0."workflowRun_status_enum" DEFAULT 'NOT_STARTED'::workspace_6n57znb4w89f8r7xudebmf0."workflowRun_status_enum" NOT NULL,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."workflowRun_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."workflowRun_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."workflowRun_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."workflowRun_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    state jsonb NOT NULL,
    "stepLogs" jsonb,
    "coreWorkflowId" uuid,
    "coreWorkflowVersionId" uuid,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, COALESCE(public.unaccent_immutable(name), ''::text))) STORED,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: workspaceMember; Type: TABLE; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE TABLE workspace_6n57znb4w89f8r7xudebmf0."workspaceMember" (
    id uuid DEFAULT public.uuid_generate_v4() NOT NULL,
    "createdAt" timestamp with time zone DEFAULT now() NOT NULL,
    "updatedAt" timestamp with time zone DEFAULT now() NOT NULL,
    "deletedAt" timestamp with time zone,
    "position" double precision DEFAULT '0'::double precision NOT NULL,
    "nameFirstName" text,
    "nameLastName" text,
    "colorScheme" text DEFAULT 'System'::text NOT NULL,
    "uiScale" text DEFAULT 'Default'::text NOT NULL,
    "openRecordIn" text DEFAULT 'SIDE_PANEL'::text NOT NULL,
    locale text DEFAULT 'en'::text NOT NULL,
    "avatarUrl" text,
    "userEmail" text,
    "jobTitle" text,
    "calendarStartDay" double precision DEFAULT '7'::double precision NOT NULL,
    "userId" uuid NOT NULL,
    "timeZone" text DEFAULT 'system'::text NOT NULL,
    "dateFormat" workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_dateFormat_enum" DEFAULT 'SYSTEM'::workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_dateFormat_enum" NOT NULL,
    "timeFormat" workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_timeFormat_enum" DEFAULT 'SYSTEM'::workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_timeFormat_enum" NOT NULL,
    "numberFormat" workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_numberFormat_enum" DEFAULT 'SYSTEM'::workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_numberFormat_enum" NOT NULL,
    "searchVector" tsvector GENERATED ALWAYS AS (to_tsvector('simple'::regconfig, ((((COALESCE(public.unaccent_immutable("nameFirstName"), ''::text) || ' '::text) || COALESCE(public.unaccent_immutable("nameLastName"), ''::text)) || ' '::text) || COALESCE(public.unaccent_immutable("userEmail"), ''::text)))) STORED,
    "createdBySource" workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_createdBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_createdBySource_enum",
    "createdByWorkspaceMemberId" uuid,
    "createdByName" text DEFAULT 'System'::text,
    "createdByContext" jsonb,
    "updatedBySource" workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_updatedBySource_enum" DEFAULT 'MANUAL'::workspace_6n57znb4w89f8r7xudebmf0."workspaceMember_updatedBySource_enum",
    "updatedByWorkspaceMemberId" uuid,
    "updatedByName" text DEFAULT 'System'::text,
    "updatedByContext" jsonb,
    "_accessGroupIds" integer[],
    "_isPrivate" boolean DEFAULT false NOT NULL
);


--
-- Name: accessGroup accessGroup_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."accessGroup"
    ADD CONSTRAINT "accessGroup_pkey" PRIMARY KEY (id);


--
-- Name: agentChatThreadParticipant agentChatThreadParticipant_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadParticipant"
    ADD CONSTRAINT "agentChatThreadParticipant_pkey" PRIMARY KEY (id);


--
-- Name: agentChatThreadTarget agentChatThreadTarget_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget"
    ADD CONSTRAINT "agentChatThreadTarget_pkey" PRIMARY KEY (id);


--
-- Name: agentChatThread agentChatThread_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThread"
    ADD CONSTRAINT "agentChatThread_pkey" PRIMARY KEY (id);


--
-- Name: agentMessagePart agentMessagePart_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentMessagePart"
    ADD CONSTRAINT "agentMessagePart_pkey" PRIMARY KEY (id);


--
-- Name: agentMessageReaction agentMessageReaction_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentMessageReaction"
    ADD CONSTRAINT "agentMessageReaction_pkey" PRIMARY KEY (id);


--
-- Name: agentMessage agentMessage_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentMessage"
    ADD CONSTRAINT "agentMessage_pkey" PRIMARY KEY (id);


--
-- Name: agentTurn agentTurn_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentTurn"
    ADD CONSTRAINT "agentTurn_pkey" PRIMARY KEY (id);


--
-- Name: attachment attachment_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.attachment
    ADD CONSTRAINT attachment_pkey PRIMARY KEY (id);


--
-- Name: blocklist blocklist_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.blocklist
    ADD CONSTRAINT blocklist_pkey PRIMARY KEY (id);


--
-- Name: calendarChannelEventAssociation calendarChannelEventAssociation_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation"
    ADD CONSTRAINT "calendarChannelEventAssociation_pkey" PRIMARY KEY (id);


--
-- Name: calendarEventParticipant calendarEventParticipant_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant"
    ADD CONSTRAINT "calendarEventParticipant_pkey" PRIMARY KEY (id);


--
-- Name: calendarEventTarget calendarEventTarget_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget"
    ADD CONSTRAINT "calendarEventTarget_pkey" PRIMARY KEY (id);


--
-- Name: calendarEvent calendarEvent_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEvent"
    ADD CONSTRAINT "calendarEvent_pkey" PRIMARY KEY (id);


--
-- Name: callRecording callRecording_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."callRecording"
    ADD CONSTRAINT "callRecording_pkey" PRIMARY KEY (id);


--
-- Name: campaignDelivery campaignDelivery_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."campaignDelivery"
    ADD CONSTRAINT "campaignDelivery_pkey" PRIMARY KEY (id);


--
-- Name: company company_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.company
    ADD CONSTRAINT company_pkey PRIMARY KEY (id);


--
-- Name: dashboard dashboard_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.dashboard
    ADD CONSTRAINT dashboard_pkey PRIMARY KEY (id);


--
-- Name: messageCampaign messageCampaign_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageCampaign"
    ADD CONSTRAINT "messageCampaign_pkey" PRIMARY KEY (id);


--
-- Name: messageChannelMessageAssociationMessageFolder messageChannelMessageAssociationMessageFolder_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder"
    ADD CONSTRAINT "messageChannelMessageAssociationMessageFolder_pkey" PRIMARY KEY (id);


--
-- Name: messageChannelMessageAssociation messageChannelMessageAssociation_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation"
    ADD CONSTRAINT "messageChannelMessageAssociation_pkey" PRIMARY KEY (id);


--
-- Name: messageListMember messageListMember_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageListMember"
    ADD CONSTRAINT "messageListMember_pkey" PRIMARY KEY (id);


--
-- Name: messageList messageList_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageList"
    ADD CONSTRAINT "messageList_pkey" PRIMARY KEY (id);


--
-- Name: messageParticipant messageParticipant_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageParticipant"
    ADD CONSTRAINT "messageParticipant_pkey" PRIMARY KEY (id);


--
-- Name: messageSuppression messageSuppression_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageSuppression"
    ADD CONSTRAINT "messageSuppression_pkey" PRIMARY KEY (id);


--
-- Name: messageThreadTarget messageThreadTarget_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget"
    ADD CONSTRAINT "messageThreadTarget_pkey" PRIMARY KEY (id);


--
-- Name: messageThread messageThread_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageThread"
    ADD CONSTRAINT "messageThread_pkey" PRIMARY KEY (id);


--
-- Name: message message_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.message
    ADD CONSTRAINT message_pkey PRIMARY KEY (id);


--
-- Name: noteTarget noteTarget_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."noteTarget"
    ADD CONSTRAINT "noteTarget_pkey" PRIMARY KEY (id);


--
-- Name: note note_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.note
    ADD CONSTRAINT note_pkey PRIMARY KEY (id);


--
-- Name: opportunity opportunity_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.opportunity
    ADD CONSTRAINT opportunity_pkey PRIMARY KEY (id);


--
-- Name: person person_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.person
    ADD CONSTRAINT person_pkey PRIMARY KEY (id);


--
-- Name: recordShare recordShare_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."recordShare"
    ADD CONSTRAINT "recordShare_pkey" PRIMARY KEY (id);


--
-- Name: shortLink shortLink_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."shortLink"
    ADD CONSTRAINT "shortLink_pkey" PRIMARY KEY (id);


--
-- Name: taskTarget taskTarget_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."taskTarget"
    ADD CONSTRAINT "taskTarget_pkey" PRIMARY KEY (id);


--
-- Name: task task_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.task
    ADD CONSTRAINT task_pkey PRIMARY KEY (id);


--
-- Name: timelineActivity timelineActivity_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "timelineActivity_pkey" PRIMARY KEY (id);


--
-- Name: workflowRun workflowRun_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."workflowRun"
    ADD CONSTRAINT "workflowRun_pkey" PRIMARY KEY (id);


--
-- Name: workspaceMember workspaceMember_pkey; Type: CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"
    ADD CONSTRAINT "workspaceMember_pkey" PRIMARY KEY (id);


--
-- Name: IDX_006c9e63c08353053e46e6f0966; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_006c9e63c08353053e46e6f0966" ON workspace_6n57znb4w89f8r7xudebmf0."messageParticipant" USING btree ("messageCampaignId");


--
-- Name: IDX_04d8ad72536bbeb4430998211f8; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_04d8ad72536bbeb4430998211f8" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING btree ("targetPersonId");


--
-- Name: IDX_1094cdc601242db548c93bb9f9a; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_1094cdc601242db548c93bb9f9a" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessage" USING btree ("threadId");


--
-- Name: IDX_126b38d940fc5029aad4149cb5a; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_126b38d940fc5029aad4149cb5a" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("targetMessageCampaignId");


--
-- Name: IDX_13be808510150ca50bd11943549; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_13be808510150ca50bd11943549" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessage" USING btree ("agentId");


--
-- Name: IDX_19ad12ae96a5d4357ff3a15ba2b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_19ad12ae96a5d4357ff3a15ba2b" ON workspace_6n57znb4w89f8r7xudebmf0."messageParticipant" USING btree ("messageId");


--
-- Name: IDX_1db95a87400f7679efc43a15e68; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_1db95a87400f7679efc43a15e68" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder" USING btree ("messageFolderId");


--
-- Name: IDX_1e9645a0d30e8ed309abf7bbfe1; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_1e9645a0d30e8ed309abf7bbfe1" ON workspace_6n57znb4w89f8r7xudebmf0."messageCampaign" USING btree ("unsubscribeTopicId");


--
-- Name: IDX_1f72eb0faade7adfd7cd96a5e16; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_1f72eb0faade7adfd7cd96a5e16" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("linkedRecordId");


--
-- Name: IDX_1ff229b8237e8368a92c08d11f0; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_1ff229b8237e8368a92c08d11f0" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("targetTaskId");


--
-- Name: IDX_21649fe7fe250834d1dd37125b1; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_21649fe7fe250834d1dd37125b1" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING btree ("targetCompanyId");


--
-- Name: IDX_261d8661b94dbb98cc85cffab46; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_261d8661b94dbb98cc85cffab46" ON workspace_6n57znb4w89f8r7xudebmf0."workflowRun" USING gin ("searchVector");


--
-- Name: IDX_26a021921ba73b428be8f244ded; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_26a021921ba73b428be8f244ded" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("targetNoteId");


--
-- Name: IDX_2e85541b739066142845bdef99a; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_2e85541b739066142845bdef99a" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation" USING btree ("messageChannelId");


--
-- Name: IDX_2f07fa4cf183061692f8d99df80; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_2f07fa4cf183061692f8d99df80" ON workspace_6n57znb4w89f8r7xudebmf0."noteTarget" USING btree ("targetCompanyId");


--
-- Name: IDX_3491dc65669342b9bfb1637f5e0; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_3491dc65669342b9bfb1637f5e0" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING btree ("targetTaskId");


--
-- Name: IDX_36a243090edb9abec66fdaf501b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_36a243090edb9abec66fdaf501b" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThread" USING btree ("channelId", "lastActivityAt");


--
-- Name: IDX_3794d47209882e7c43f4f786f33; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_3794d47209882e7c43f4f786f33" ON workspace_6n57znb4w89f8r7xudebmf0."accessGroup" USING btree ("objectMetadataId");


--
-- Name: IDX_3f4c0095cf17b62868bec089fab; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_3f4c0095cf17b62868bec089fab" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation" USING btree ("messageId");


--
-- Name: IDX_3fa2da2cb337b36955206551cf9; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_3fa2da2cb337b36955206551cf9" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("targetMessageListId");


--
-- Name: IDX_40150517e4f1ab6154e426eafce; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_40150517e4f1ab6154e426eafce" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant" USING btree ("calendarEventId");


--
-- Name: IDX_4036b3fcba7a72c35b74c91d0cb; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_4036b3fcba7a72c35b74c91d0cb" ON workspace_6n57znb4w89f8r7xudebmf0.message USING btree ("messageCampaignId");


--
-- Name: IDX_40d619d1dff8a2fd54a8db17d87; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_40d619d1dff8a2fd54a8db17d87" ON workspace_6n57znb4w89f8r7xudebmf0."messageListMember" USING btree ("listId");


--
-- Name: IDX_419778384cd97935db3e246f589; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_419778384cd97935db3e246f589" ON workspace_6n57znb4w89f8r7xudebmf0."taskTarget" USING btree ("targetCompanyId");


--
-- Name: IDX_436a8c97506b504cd12f8058d68; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_436a8c97506b504cd12f8058d68" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEvent" USING btree ("iCalUid");


--
-- Name: IDX_46d260f179659e359685d025b55; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_46d260f179659e359685d025b55" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessagePart" USING btree ("messageId", "orderIndex");


--
-- Name: IDX_47a5ea9e149973d6ef980bdc4f1; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_47a5ea9e149973d6ef980bdc4f1" ON workspace_6n57znb4w89f8r7xudebmf0."messageParticipant" USING btree ("personId");


--
-- Name: IDX_4a277b3dad18715d2619fee7c5b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_4a277b3dad18715d2619fee7c5b" ON workspace_6n57znb4w89f8r7xudebmf0."recordShare" USING btree ("principalId");


--
-- Name: IDX_4b9feee3298c853326bf6ff8e42; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_4b9feee3298c853326bf6ff8e42" ON workspace_6n57znb4w89f8r7xudebmf0.opportunity USING btree ("companyId");


--
-- Name: IDX_4c7e63468918a5d5233ccb65de7; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_4c7e63468918a5d5233ccb65de7" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING btree ("targetDashboardId");


--
-- Name: IDX_543763e64093e0f69a67143ed76; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_543763e64093e0f69a67143ed76" ON workspace_6n57znb4w89f8r7xudebmf0.message USING btree ("headerMessageId");


--
-- Name: IDX_5519b21a3b491ab8b1ce594b119; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_5519b21a3b491ab8b1ce594b119" ON workspace_6n57znb4w89f8r7xudebmf0."recordShare" USING btree ("sourceId");


--
-- Name: IDX_56d88cda062c63919db0ee7709d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_56d88cda062c63919db0ee7709d" ON workspace_6n57znb4w89f8r7xudebmf0."messageCampaign" USING btree ("listId");


--
-- Name: IDX_58b130a455b1451066c84dc63e2; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_58b130a455b1451066c84dc63e2" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("targetWorkflowRunId");


--
-- Name: IDX_5d002cd3b5be1cb05c0b7b28582; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_5d002cd3b5be1cb05c0b7b28582" ON workspace_6n57znb4w89f8r7xudebmf0.message USING btree ("messageThreadId");


--
-- Name: IDX_6096ed9e3b57556811612066e60; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_6096ed9e3b57556811612066e60" ON workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" USING btree ("messageThreadId");


--
-- Name: IDX_62d09af534224aa478109b2d585; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_62d09af534224aa478109b2d585" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("targetOpportunityId");


--
-- Name: IDX_64d57b165b3cd64f71fe8a0ea4c; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_64d57b165b3cd64f71fe8a0ea4c" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING btree ("targetAgentChatThreadId");


--
-- Name: IDX_699ff395576ff0f73cc87464166; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_699ff395576ff0f73cc87464166" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING btree ("targetNoteId");


--
-- Name: IDX_6b2a27852dd0e0846577662a33a; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_6b2a27852dd0e0846577662a33a" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("targetDashboardId");


--
-- Name: IDX_70c8cfc3c0c8407789db32ad9cf; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_70c8cfc3c0c8407789db32ad9cf" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant" USING btree ("workspaceMemberId");


--
-- Name: IDX_733381453fca683f36c05af5478; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_733381453fca683f36c05af5478" ON workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation" USING btree ("calendarEventId");


--
-- Name: IDX_746238bc7fc291401a93df1e144; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_746238bc7fc291401a93df1e144" ON workspace_6n57znb4w89f8r7xudebmf0."callRecording" USING btree ("calendarEventId");


--
-- Name: IDX_74ad70941560ba6b2a179ad460c; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_74ad70941560ba6b2a179ad460c" ON workspace_6n57znb4w89f8r7xudebmf0."taskTarget" USING btree ("taskId");


--
-- Name: IDX_7579399b4a20615f1b1e4b06f61; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_7579399b4a20615f1b1e4b06f61" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThread" USING btree ("workspaceMemberId");


--
-- Name: IDX_7a56509bca48a709378017a9135; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_7a56509bca48a709378017a9135" ON workspace_6n57znb4w89f8r7xudebmf0."messageParticipant" USING btree ("workspaceMemberId");


--
-- Name: IDX_7bbbea99393fa98ed11835f096d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_7bbbea99393fa98ed11835f096d" ON workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" USING btree ("targetPersonId");


--
-- Name: IDX_7e2582241f3b749d7a43d7d0231; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_7e2582241f3b749d7a43d7d0231" ON workspace_6n57znb4w89f8r7xudebmf0."noteTarget" USING btree ("noteId");


--
-- Name: IDX_7f4bdb408d2be2acc757c58b122; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_7f4bdb408d2be2acc757c58b122" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadParticipant" USING btree ("workspaceMemberId");


--
-- Name: IDX_84069e90156efdc5c72534df193; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_84069e90156efdc5c72534df193" ON workspace_6n57znb4w89f8r7xudebmf0."workflowRun" USING btree ("coreWorkflowId", "createdAt");


--
-- Name: IDX_84ba2c3b79b6421fb7a5f55df28; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_84ba2c3b79b6421fb7a5f55df28" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessageReaction" USING btree ("workspaceMemberId");


--
-- Name: IDX_883e4188651697bc2a3d765462c; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_883e4188651697bc2a3d765462c" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" USING btree ("threadId");


--
-- Name: IDX_8aef81915781c02a4072c572f24; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_8aef81915781c02a4072c572f24" ON workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" USING btree ("targetCompanyId");


--
-- Name: IDX_8b39f43d7bc0f901fcb343cf5e0; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_8b39f43d7bc0f901fcb343cf5e0" ON workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" USING btree ("targetOpportunityId");


--
-- Name: IDX_8b3b9fa5cb58ad45fd61eba8c6c; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_8b3b9fa5cb58ad45fd61eba8c6c" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessage" USING btree ("turnId");


--
-- Name: IDX_8f0e8249c3a27735c4ab5e1f30d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_8f0e8249c3a27735c4ab5e1f30d" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" USING btree ("targetPersonId");


--
-- Name: IDX_9058210268a941b4b77a609e982; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_9058210268a941b4b77a609e982" ON workspace_6n57znb4w89f8r7xudebmf0."noteTarget" USING btree ("targetOpportunityId");


--
-- Name: IDX_9566d8b8725ff67e68363574be6; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_9566d8b8725ff67e68363574be6" ON workspace_6n57znb4w89f8r7xudebmf0."agentTurn" USING btree ("threadId");


--
-- Name: IDX_968d4fd721a78b75c13a1b9ec12; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_968d4fd721a78b75c13a1b9ec12" ON workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation" USING btree ("calendarChannelId");


--
-- Name: IDX_9b267fc4a89dd1aaec4c5340f05; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_9b267fc4a89dd1aaec4c5340f05" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("targetCompanyId");


--
-- Name: IDX_9e247b4ab168100e4aa8fb6a853; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_9e247b4ab168100e4aa8fb6a853" ON workspace_6n57znb4w89f8r7xudebmf0.blocklist USING btree ("workspaceMemberId");


--
-- Name: IDX_9f96d65260c4676faac27cb6bf3; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_9f96d65260c4676faac27cb6bf3" ON workspace_6n57znb4w89f8r7xudebmf0.opportunity USING gin ("searchVector");


--
-- Name: IDX_ACCESS_GROUP_COMPACT_ID_UNIQUE; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_ACCESS_GROUP_COMPACT_ID_UNIQUE" ON workspace_6n57znb4w89f8r7xudebmf0."accessGroup" USING btree ("compactId");


--
-- Name: IDX_ACCESS_GROUP_IDS_03e66fc684884f3cae55de5db3f0b119; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_03e66fc684884f3cae55de5db3f0b119" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessage" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_0f5d2c7448a444acb1b319f6872834c1; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_0f5d2c7448a444acb1b319f6872834c1" ON workspace_6n57znb4w89f8r7xudebmf0."messageParticipant" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_10973f5f054940a2800069e250b32c6d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_10973f5f054940a2800069e250b32c6d" ON workspace_6n57znb4w89f8r7xudebmf0.message USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_113205d58ad64af1b29f0cd82d48d90f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_113205d58ad64af1b29f0cd82d48d90f" ON workspace_6n57znb4w89f8r7xudebmf0."messageSuppression" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_12a873f3d09d4244ba4c96330de68d63; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_12a873f3d09d4244ba4c96330de68d63" ON workspace_6n57znb4w89f8r7xudebmf0."messageThread" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_1486bc1a8de844a08a487ce705c652aa; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_1486bc1a8de844a08a487ce705c652aa" ON workspace_6n57znb4w89f8r7xudebmf0.task USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_16116f21b2d848929b8517fc672d555a; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_16116f21b2d848929b8517fc672d555a" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_1f8e4a39ab5e478a9bfbb6346b654c6e; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_1f8e4a39ab5e478a9bfbb6346b654c6e" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_28030f4c50644fdab1f04dd2a5efc60e; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_28030f4c50644fdab1f04dd2a5efc60e" ON workspace_6n57znb4w89f8r7xudebmf0."messageCampaign" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_3547572f721d4d3eac7e929e99f6c375; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_3547572f721d4d3eac7e929e99f6c375" ON workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_3a80a686ab4845daab7562d40be34cd7; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_3a80a686ab4845daab7562d40be34cd7" ON workspace_6n57znb4w89f8r7xudebmf0."taskTarget" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_528c21c3d6024810809bda1b965a28e0; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_528c21c3d6024810809bda1b965a28e0" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThread" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_53970ec4820a48eda698c822756f07fa; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_53970ec4820a48eda698c822756f07fa" ON workspace_6n57znb4w89f8r7xudebmf0."noteTarget" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_6160317a324f4dbcbcd5ef07a779273b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_6160317a324f4dbcbcd5ef07a779273b" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_619fa0dcb3064072a4d83a6dff1bb498; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_619fa0dcb3064072a4d83a6dff1bb498" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadParticipant" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_64118c3616aa47f0b0140c1be44fb1fd; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_64118c3616aa47f0b0140c1be44fb1fd" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_6870246ad0ff4464949b6d9501dc1a72; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_6870246ad0ff4464949b6d9501dc1a72" ON workspace_6n57znb4w89f8r7xudebmf0.person USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_6f6acda348964eaf9fe334dc9a27cdcb; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_6f6acda348964eaf9fe334dc9a27cdcb" ON workspace_6n57znb4w89f8r7xudebmf0."workflowRun" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_778a6ed5cd5b404fabf542d99315344d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_778a6ed5cd5b404fabf542d99315344d" ON workspace_6n57znb4w89f8r7xudebmf0."messageListMember" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_7c39c7b98ed54fec9547ed91895cbbe0; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_7c39c7b98ed54fec9547ed91895cbbe0" ON workspace_6n57znb4w89f8r7xudebmf0."workspaceMember" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_8a8d2f0e59ce4da5ab659eae93dbd56d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_8a8d2f0e59ce4da5ab659eae93dbd56d" ON workspace_6n57znb4w89f8r7xudebmf0.dashboard USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_8d229465aade4b3ca4d71cda43015b80; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_8d229465aade4b3ca4d71cda43015b80" ON workspace_6n57znb4w89f8r7xudebmf0."campaignDelivery" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_8d82d7bbb7de4031ab8dcf97873ea00c; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_8d82d7bbb7de4031ab8dcf97873ea00c" ON workspace_6n57znb4w89f8r7xudebmf0."shortLink" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_984f417d99e94ac7a956b686c10a8f2f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_984f417d99e94ac7a956b686c10a8f2f" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessagePart" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_9a0b91aebc0b4c6a81751c6bd80fdf0a; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_9a0b91aebc0b4c6a81751c6bd80fdf0a" ON workspace_6n57znb4w89f8r7xudebmf0."callRecording" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_a3df70ef88df4cefb796eb8715178e96; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_a3df70ef88df4cefb796eb8715178e96" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEvent" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_a57363357c6c4638947d714ee3961786; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_a57363357c6c4638947d714ee3961786" ON workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_b3772901a2b34557b6315318b996492f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_b3772901a2b34557b6315318b996492f" ON workspace_6n57znb4w89f8r7xudebmf0.opportunity USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_c39ec311db8f41b0bf11184c8a57c18f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_c39ec311db8f41b0bf11184c8a57c18f" ON workspace_6n57znb4w89f8r7xudebmf0."agentTurn" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_c75f538ac1ff4c2f870ea740422d4528; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_c75f538ac1ff4c2f870ea740422d4528" ON workspace_6n57znb4w89f8r7xudebmf0.note USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_ca67c4bcb75b4038b2550a80ea6141cb; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_ca67c4bcb75b4038b2550a80ea6141cb" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_d2f5199170ed46fb9dcf928955500a8c; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_d2f5199170ed46fb9dcf928955500a8c" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_d54a33261f374de193560be3b8559e29; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_d54a33261f374de193560be3b8559e29" ON workspace_6n57znb4w89f8r7xudebmf0."messageList" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_de846a98fb7a40be83dfb417e15c4c97; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_de846a98fb7a40be83dfb417e15c4c97" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_f3bed6b8fd9940de88453d6254206a02; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_f3bed6b8fd9940de88453d6254206a02" ON workspace_6n57znb4w89f8r7xudebmf0.company USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_f41d9242cd0e441a9ccd9eb9508e4bc1; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_f41d9242cd0e441a9ccd9eb9508e4bc1" ON workspace_6n57znb4w89f8r7xudebmf0.blocklist USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_IDS_ff16c7897bc3419fa176c53a68c25a25; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_IDS_ff16c7897bc3419fa176c53a68c25a25" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessageReaction" USING gin ("_accessGroupIds");


--
-- Name: IDX_ACCESS_GROUP_RULE_SHAPE_MEMBER_FIELD_VALUES; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_RULE_SHAPE_MEMBER_FIELD_VALUES" ON workspace_6n57znb4w89f8r7xudebmf0."accessGroup" USING btree ("ruleShapeHash", "memberFieldValues") WHERE (type = 'RLS_MEMBER_VALUE'::workspace_6n57znb4w89f8r7xudebmf0."accessGroup_type_enum");


--
-- Name: IDX_ACCESS_GROUP_RULE_SHAPE_VALUE; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ACCESS_GROUP_RULE_SHAPE_VALUE" ON workspace_6n57znb4w89f8r7xudebmf0."accessGroup" USING btree ("ruleShapeHash", value) WHERE (type = 'RLS_LITERAL_VALUE'::workspace_6n57znb4w89f8r7xudebmf0."accessGroup_type_enum");


--
-- Name: IDX_PRIVATE_RECORD_03e66fc684884f3cae55de5db3f0b119; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_03e66fc684884f3cae55de5db3f0b119" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessage" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_0f5d2c7448a444acb1b319f6872834c1; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_0f5d2c7448a444acb1b319f6872834c1" ON workspace_6n57znb4w89f8r7xudebmf0."messageParticipant" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_10973f5f054940a2800069e250b32c6d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_10973f5f054940a2800069e250b32c6d" ON workspace_6n57znb4w89f8r7xudebmf0.message USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_113205d58ad64af1b29f0cd82d48d90f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_113205d58ad64af1b29f0cd82d48d90f" ON workspace_6n57znb4w89f8r7xudebmf0."messageSuppression" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_12a873f3d09d4244ba4c96330de68d63; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_12a873f3d09d4244ba4c96330de68d63" ON workspace_6n57znb4w89f8r7xudebmf0."messageThread" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_1486bc1a8de844a08a487ce705c652aa; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_1486bc1a8de844a08a487ce705c652aa" ON workspace_6n57znb4w89f8r7xudebmf0.task USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_16116f21b2d848929b8517fc672d555a; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_16116f21b2d848929b8517fc672d555a" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_1f8e4a39ab5e478a9bfbb6346b654c6e; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_1f8e4a39ab5e478a9bfbb6346b654c6e" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_28030f4c50644fdab1f04dd2a5efc60e; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_28030f4c50644fdab1f04dd2a5efc60e" ON workspace_6n57znb4w89f8r7xudebmf0."messageCampaign" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_3547572f721d4d3eac7e929e99f6c375; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_3547572f721d4d3eac7e929e99f6c375" ON workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_3a80a686ab4845daab7562d40be34cd7; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_3a80a686ab4845daab7562d40be34cd7" ON workspace_6n57znb4w89f8r7xudebmf0."taskTarget" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_528c21c3d6024810809bda1b965a28e0; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_528c21c3d6024810809bda1b965a28e0" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThread" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_53970ec4820a48eda698c822756f07fa; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_53970ec4820a48eda698c822756f07fa" ON workspace_6n57znb4w89f8r7xudebmf0."noteTarget" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_6160317a324f4dbcbcd5ef07a779273b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_6160317a324f4dbcbcd5ef07a779273b" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_619fa0dcb3064072a4d83a6dff1bb498; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_619fa0dcb3064072a4d83a6dff1bb498" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadParticipant" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_64118c3616aa47f0b0140c1be44fb1fd; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_64118c3616aa47f0b0140c1be44fb1fd" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_6870246ad0ff4464949b6d9501dc1a72; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_6870246ad0ff4464949b6d9501dc1a72" ON workspace_6n57znb4w89f8r7xudebmf0.person USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_6f6acda348964eaf9fe334dc9a27cdcb; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_6f6acda348964eaf9fe334dc9a27cdcb" ON workspace_6n57znb4w89f8r7xudebmf0."workflowRun" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_778a6ed5cd5b404fabf542d99315344d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_778a6ed5cd5b404fabf542d99315344d" ON workspace_6n57znb4w89f8r7xudebmf0."messageListMember" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_7c39c7b98ed54fec9547ed91895cbbe0; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_7c39c7b98ed54fec9547ed91895cbbe0" ON workspace_6n57znb4w89f8r7xudebmf0."workspaceMember" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_8a8d2f0e59ce4da5ab659eae93dbd56d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_8a8d2f0e59ce4da5ab659eae93dbd56d" ON workspace_6n57znb4w89f8r7xudebmf0.dashboard USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_8d229465aade4b3ca4d71cda43015b80; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_8d229465aade4b3ca4d71cda43015b80" ON workspace_6n57znb4w89f8r7xudebmf0."campaignDelivery" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_8d82d7bbb7de4031ab8dcf97873ea00c; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_8d82d7bbb7de4031ab8dcf97873ea00c" ON workspace_6n57znb4w89f8r7xudebmf0."shortLink" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_984f417d99e94ac7a956b686c10a8f2f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_984f417d99e94ac7a956b686c10a8f2f" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessagePart" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_9a0b91aebc0b4c6a81751c6bd80fdf0a; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_9a0b91aebc0b4c6a81751c6bd80fdf0a" ON workspace_6n57znb4w89f8r7xudebmf0."callRecording" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_a3df70ef88df4cefb796eb8715178e96; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_a3df70ef88df4cefb796eb8715178e96" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEvent" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_a57363357c6c4638947d714ee3961786; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_a57363357c6c4638947d714ee3961786" ON workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_b3772901a2b34557b6315318b996492f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_b3772901a2b34557b6315318b996492f" ON workspace_6n57znb4w89f8r7xudebmf0.opportunity USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_c39ec311db8f41b0bf11184c8a57c18f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_c39ec311db8f41b0bf11184c8a57c18f" ON workspace_6n57znb4w89f8r7xudebmf0."agentTurn" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_c75f538ac1ff4c2f870ea740422d4528; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_c75f538ac1ff4c2f870ea740422d4528" ON workspace_6n57znb4w89f8r7xudebmf0.note USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_ca67c4bcb75b4038b2550a80ea6141cb; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_ca67c4bcb75b4038b2550a80ea6141cb" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_d2f5199170ed46fb9dcf928955500a8c; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_d2f5199170ed46fb9dcf928955500a8c" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_d54a33261f374de193560be3b8559e29; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_d54a33261f374de193560be3b8559e29" ON workspace_6n57znb4w89f8r7xudebmf0."messageList" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_de846a98fb7a40be83dfb417e15c4c97; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_de846a98fb7a40be83dfb417e15c4c97" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_f3bed6b8fd9940de88453d6254206a02; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_f3bed6b8fd9940de88453d6254206a02" ON workspace_6n57znb4w89f8r7xudebmf0.company USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_f41d9242cd0e441a9ccd9eb9508e4bc1; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_f41d9242cd0e441a9ccd9eb9508e4bc1" ON workspace_6n57znb4w89f8r7xudebmf0.blocklist USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_PRIVATE_RECORD_ff16c7897bc3419fa176c53a68c25a25; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_PRIVATE_RECORD_ff16c7897bc3419fa176c53a68c25a25" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessageReaction" USING btree (id) WHERE "_isPrivate";


--
-- Name: IDX_UNIQUE_06b56491f0f07b397a2593de27c; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_06b56491f0f07b397a2593de27c" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" USING btree ("threadId", "targetPersonId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_10f41e6e837c3c40fa07ca9d76f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_10f41e6e837c3c40fa07ca9d76f" ON workspace_6n57znb4w89f8r7xudebmf0.company USING btree ("domainNamePrimaryLinkUrl") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_14866214efbeb5ae8a79feac12d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_14866214efbeb5ae8a79feac12d" ON workspace_6n57znb4w89f8r7xudebmf0."messageSuppression" USING btree ("emailAddress", "unsubscribeTopicId");


--
-- Name: IDX_UNIQUE_1d5a5eaa9988b2094229723149a; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_1d5a5eaa9988b2094229723149a" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessageReaction" USING btree ("messageId", "workspaceMemberId", emoji) WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_20768e530f061d3d69d769c4086; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_20768e530f061d3d69d769c4086" ON workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" USING btree ("messageThreadId", "targetCompanyId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_2a8112c19cab880dde4b9b122e0; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_2a8112c19cab880dde4b9b122e0" ON workspace_6n57znb4w89f8r7xudebmf0."recordShare" USING btree ("objectMetadataId", "recordId", "principalId", "rowCause", "sourceId");


--
-- Name: IDX_UNIQUE_2e7fe677ecee1914250716dd94e; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_2e7fe677ecee1914250716dd94e" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThread" USING btree ("directMessageFirstWorkspaceMemberId", "directMessageSecondWorkspaceMemberId");


--
-- Name: IDX_UNIQUE_356945c0521c3380d2fcfd42ecf; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_356945c0521c3380d2fcfd42ecf" ON workspace_6n57znb4w89f8r7xudebmf0."taskTarget" USING btree ("taskId", "targetOpportunityId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_3726a3647f8a77e68a8b9d37c6b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_3726a3647f8a77e68a8b9d37c6b" ON workspace_6n57znb4w89f8r7xudebmf0."campaignDelivery" USING btree ("providerMessageId");


--
-- Name: IDX_UNIQUE_5b7dc38ea61fa3881934d8d9c30; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_5b7dc38ea61fa3881934d8d9c30" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" USING btree ("threadId", "targetOpportunityId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_75e00e7c3d31b534e3817684e7b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_75e00e7c3d31b534e3817684e7b" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" USING btree ("calendarEventId", "targetOpportunityId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_7c957b135288e84154710892971; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_7c957b135288e84154710892971" ON workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" USING btree ("messageThreadId", "targetOpportunityId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_7d44a0a2eb17d4a62633fdada52; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_7d44a0a2eb17d4a62633fdada52" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" USING btree ("threadId", "targetCompanyId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_8a81f90474e54b2a831afb20864; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_8a81f90474e54b2a831afb20864" ON workspace_6n57znb4w89f8r7xudebmf0.person USING btree ("emailsPrimaryEmail") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_8b23e16629403e4eeef7178f9f6; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_8b23e16629403e4eeef7178f9f6" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation" USING btree ("messageChannelId", "messageId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_96a710d1ea663b548a879a47751; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_96a710d1ea663b548a879a47751" ON workspace_6n57znb4w89f8r7xudebmf0."shortLink" USING btree ("templateAndResolvedUrlHash");


--
-- Name: IDX_UNIQUE_9bf8fcb425674d0378395973a15; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_9bf8fcb425674d0378395973a15" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" USING btree ("calendarEventId", "targetPersonId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_9da382bc767f8645e027a4ede79; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_9da382bc767f8645e027a4ede79" ON workspace_6n57znb4w89f8r7xudebmf0."campaignDelivery" USING btree ("campaignId", "personId");


--
-- Name: IDX_UNIQUE_a4b191d45acc856a8561093dc3b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_a4b191d45acc856a8561093dc3b" ON workspace_6n57znb4w89f8r7xudebmf0."messageListMember" USING btree ("personId", "listId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_a84a588c9b4600b1226535d4eca; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_a84a588c9b4600b1226535d4eca" ON workspace_6n57znb4w89f8r7xudebmf0."taskTarget" USING btree ("taskId", "targetCompanyId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_aba87cbda880a133d7bf08867c1; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_aba87cbda880a133d7bf08867c1" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadParticipant" USING btree ("threadId", "workspaceMemberId");


--
-- Name: IDX_UNIQUE_af63af1fe284f1d0475a1a2a64b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_af63af1fe284f1d0475a1a2a64b" ON workspace_6n57znb4w89f8r7xudebmf0."noteTarget" USING btree ("noteId", "targetPersonId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_b784cebdb5d8238240a4b99b8e2; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_b784cebdb5d8238240a4b99b8e2" ON workspace_6n57znb4w89f8r7xudebmf0."accessGroup" USING btree ("groupHash");


--
-- Name: IDX_UNIQUE_bc68c7cde7013367efffd85dff6; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_bc68c7cde7013367efffd85dff6" ON workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget" USING btree ("messageThreadId", "targetPersonId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_c1de2c38251a971648808f7e6a6; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_c1de2c38251a971648808f7e6a6" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder" USING btree ("messageChannelMessageAssociationId", "messageFolderId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_c59cfcdcf0d35ddea660770926b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_c59cfcdcf0d35ddea660770926b" ON workspace_6n57znb4w89f8r7xudebmf0."taskTarget" USING btree ("taskId", "targetPersonId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_ca288a7ac39c3f7781cef6f5596; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_ca288a7ac39c3f7781cef6f5596" ON workspace_6n57znb4w89f8r7xudebmf0."workspaceMember" USING btree ("userEmail") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_d3c4f75ca45d2fa94b5860ec5fb; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_d3c4f75ca45d2fa94b5860ec5fb" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" USING btree ("calendarEventId", "targetCompanyId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_d4caca2b253642e0f16cd7c1790; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_d4caca2b253642e0f16cd7c1790" ON workspace_6n57znb4w89f8r7xudebmf0."noteTarget" USING btree ("noteId", "targetCompanyId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_UNIQUE_da8767d0e1e917b432d57881b51; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE UNIQUE INDEX "IDX_UNIQUE_da8767d0e1e917b432d57881b51" ON workspace_6n57znb4w89f8r7xudebmf0."noteTarget" USING btree ("noteId", "targetOpportunityId") WHERE ("deletedAt" IS NULL);


--
-- Name: IDX_a1e176335c31525bf3cc12f0e00; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_a1e176335c31525bf3cc12f0e00" ON workspace_6n57znb4w89f8r7xudebmf0."noteTarget" USING btree ("targetPersonId");


--
-- Name: IDX_a5dcf976f16b8ce26a74de4084d; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_a5dcf976f16b8ce26a74de4084d" ON workspace_6n57znb4w89f8r7xudebmf0."agentTurn" USING btree ("agentId");


--
-- Name: IDX_aa4205942a2300605ed6531335f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_aa4205942a2300605ed6531335f" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" USING btree ("targetOpportunityId");


--
-- Name: IDX_ae09ff97967369e0644bacc0fce; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ae09ff97967369e0644bacc0fce" ON workspace_6n57znb4w89f8r7xudebmf0.person USING btree ("companyId");


--
-- Name: IDX_ae112c10e060420923011767b14; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ae112c10e060420923011767b14" ON workspace_6n57znb4w89f8r7xudebmf0.opportunity USING btree (stage);


--
-- Name: IDX_aed8933218379206df7a39000ee; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_aed8933218379206df7a39000ee" ON workspace_6n57znb4w89f8r7xudebmf0."campaignDelivery" USING btree ("claimToken");


--
-- Name: IDX_b5b4da613fc4d734f65fb1deb6b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_b5b4da613fc4d734f65fb1deb6b" ON workspace_6n57znb4w89f8r7xudebmf0.task USING btree ("assigneeId");


--
-- Name: IDX_ba8418718688702d3113fde2fe1; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ba8418718688702d3113fde2fe1" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant" USING btree ("personId");


--
-- Name: IDX_bbd7aec1976fc684a0a5e4816c9; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_bbd7aec1976fc684a0a5e4816c9" ON workspace_6n57znb4w89f8r7xudebmf0.person USING gin ("searchVector");


--
-- Name: IDX_c0ac950d77b75527f654b7f6a06; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_c0ac950d77b75527f654b7f6a06" ON workspace_6n57znb4w89f8r7xudebmf0.opportunity USING btree ("pointOfContactId");


--
-- Name: IDX_c4973918b795497fd7e41bda0f7; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_c4973918b795497fd7e41bda0f7" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING btree ("targetMessageId");


--
-- Name: IDX_ce2463a1837c7824ffbc5af165e; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ce2463a1837c7824ffbc5af165e" ON workspace_6n57znb4w89f8r7xudebmf0."agentTurn" USING btree ("agentId", "createdAt");


--
-- Name: IDX_d01a000cf26e1225d894dc3d364; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_d01a000cf26e1225d894dc3d364" ON workspace_6n57znb4w89f8r7xudebmf0.task USING gin ("searchVector");


--
-- Name: IDX_d1c48fde83b0c0ec4a1203d88cb; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_d1c48fde83b0c0ec4a1203d88cb" ON workspace_6n57znb4w89f8r7xudebmf0."messageList" USING gin ("searchVector");


--
-- Name: IDX_d250cf8fc007e3ae5ab5c206b92; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_d250cf8fc007e3ae5ab5c206b92" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" USING btree ("targetCompanyId");


--
-- Name: IDX_d296dac8d69327aaaf9aec27d65; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_d296dac8d69327aaaf9aec27d65" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" USING btree ("targetOpportunityId");


--
-- Name: IDX_d2c427300cfaa66582eace551a6; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_d2c427300cfaa66582eace551a6" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThread" USING btree ("assigneeId");


--
-- Name: IDX_daf00d51b50634730fd77f16bb6; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_daf00d51b50634730fd77f16bb6" ON workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder" USING btree ("messageChannelMessageAssociationId");


--
-- Name: IDX_db5edd55019fc47dc95b5da52e6; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_db5edd55019fc47dc95b5da52e6" ON workspace_6n57znb4w89f8r7xudebmf0."campaignDelivery" USING btree ("campaignId", state);


--
-- Name: IDX_dc68847d0ff0b17baef8fb632c2; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_dc68847d0ff0b17baef8fb632c2" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("workspaceMemberId");


--
-- Name: IDX_df649f6cb2045b56149f294972f; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_df649f6cb2045b56149f294972f" ON workspace_6n57znb4w89f8r7xudebmf0."agentMessage" USING btree ("senderWorkspaceMemberId");


--
-- Name: IDX_e0b9688852672194885987500d0; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_e0b9688852672194885987500d0" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" USING btree ("calendarEventId");


--
-- Name: IDX_e47451872f70c8f187a6b460ac7; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_e47451872f70c8f187a6b460ac7" ON workspace_6n57znb4w89f8r7xudebmf0."workspaceMember" USING gin ("searchVector");


--
-- Name: IDX_e49dd7da4ed5c919babcd31c93b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_e49dd7da4ed5c919babcd31c93b" ON workspace_6n57znb4w89f8r7xudebmf0."timelineActivity" USING btree ("targetPersonId");


--
-- Name: IDX_e91757c0ebb42004bef8307b426; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_e91757c0ebb42004bef8307b426" ON workspace_6n57znb4w89f8r7xudebmf0."messageCampaign" USING gin ("searchVector");


--
-- Name: IDX_eb0e80b3ed3d516f60454561450; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_eb0e80b3ed3d516f60454561450" ON workspace_6n57znb4w89f8r7xudebmf0."campaignDelivery" USING btree ("claimExpiresAt");


--
-- Name: IDX_ef1566ae69f6022f77d7f11b6c5; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_ef1566ae69f6022f77d7f11b6c5" ON workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget" USING btree ("targetCompanyId");


--
-- Name: IDX_f0e95ec5e72b91f28fffac1201b; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_f0e95ec5e72b91f28fffac1201b" ON workspace_6n57znb4w89f8r7xudebmf0."taskTarget" USING btree ("targetOpportunityId");


--
-- Name: IDX_f1441ffea34b02323606d22c751; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_f1441ffea34b02323606d22c751" ON workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget" USING btree ("targetPersonId");


--
-- Name: IDX_f20de8d7fc74a405e4083051275; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_f20de8d7fc74a405e4083051275" ON workspace_6n57znb4w89f8r7xudebmf0.note USING gin ("searchVector");


--
-- Name: IDX_f34c350d237f0199b4d33298cd8; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_f34c350d237f0199b4d33298cd8" ON workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation" USING btree ("eventExternalId");


--
-- Name: IDX_f3b76c5322b31cba175b2eccec8; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_f3b76c5322b31cba175b2eccec8" ON workspace_6n57znb4w89f8r7xudebmf0.dashboard USING gin ("searchVector");


--
-- Name: IDX_f3d9b01b0d587804e7beeb2a534; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_f3d9b01b0d587804e7beeb2a534" ON workspace_6n57znb4w89f8r7xudebmf0."taskTarget" USING btree ("targetPersonId");


--
-- Name: IDX_f719a95179070eac397ba18dc70; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_f719a95179070eac397ba18dc70" ON workspace_6n57znb4w89f8r7xudebmf0.company USING btree ("accountOwnerId");


--
-- Name: IDX_fb1f4905546cfc6d70a971c76f7; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_fb1f4905546cfc6d70a971c76f7" ON workspace_6n57znb4w89f8r7xudebmf0.company USING gin ("searchVector");


--
-- Name: IDX_fde993c7020a17c3282edf5c9b8; Type: INDEX; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

CREATE INDEX "IDX_fde993c7020a17c3282edf5c9b8" ON workspace_6n57znb4w89f8r7xudebmf0.attachment USING btree ("targetOpportunityId");


--
-- Name: timelineActivity FK_0482ca18fc5e49456ce97cf61aa; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_0482ca18fc5e49456ce97cf61aa" FOREIGN KEY ("targetOpportunityId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.opportunity(id) ON DELETE SET NULL;


--
-- Name: taskTarget FK_050bf68b54d229f319d6a73ba1c; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."taskTarget"
    ADD CONSTRAINT "FK_050bf68b54d229f319d6a73ba1c" FOREIGN KEY ("targetPersonId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE CASCADE;


--
-- Name: timelineActivity FK_068146151017a5f32c3b152ac04; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_068146151017a5f32c3b152ac04" FOREIGN KEY ("targetNoteId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.note(id) ON DELETE SET NULL;


--
-- Name: calendarEventParticipant FK_082f03025609e55b3d55de7cc28; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant"
    ADD CONSTRAINT "FK_082f03025609e55b3d55de7cc28" FOREIGN KEY ("calendarEventId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."calendarEvent"(id) ON DELETE CASCADE;


--
-- Name: messageParticipant FK_097516fa4a6f3b2d206e6574fa6; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageParticipant"
    ADD CONSTRAINT "FK_097516fa4a6f3b2d206e6574fa6" FOREIGN KEY ("messageCampaignId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageCampaign"(id) ON DELETE SET NULL;


--
-- Name: attachment FK_09f23b12008e2b4f1262738b088; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.attachment
    ADD CONSTRAINT "FK_09f23b12008e2b4f1262738b088" FOREIGN KEY ("targetMessageId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.message(id) ON DELETE CASCADE;


--
-- Name: agentChatThreadParticipant FK_0eff97f12996b031e58840a8d6d; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadParticipant"
    ADD CONSTRAINT "FK_0eff97f12996b031e58840a8d6d" FOREIGN KEY ("workspaceMemberId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE CASCADE;


--
-- Name: messageListMember FK_11ef3dc93face56b812aece0637; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageListMember"
    ADD CONSTRAINT "FK_11ef3dc93face56b812aece0637" FOREIGN KEY ("personId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE CASCADE;


--
-- Name: calendarEventTarget FK_138e52d02dcfd9624f4e82abb8e; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget"
    ADD CONSTRAINT "FK_138e52d02dcfd9624f4e82abb8e" FOREIGN KEY ("targetPersonId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE CASCADE;


--
-- Name: agentChatThreadTarget FK_1a1272c63027e5ae929fbc54e2d; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget"
    ADD CONSTRAINT "FK_1a1272c63027e5ae929fbc54e2d" FOREIGN KEY ("targetOpportunityId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.opportunity(id) ON DELETE CASCADE;


--
-- Name: calendarChannelEventAssociation FK_1fc99b189521f153bf3bbac7324; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarChannelEventAssociation"
    ADD CONSTRAINT "FK_1fc99b189521f153bf3bbac7324" FOREIGN KEY ("calendarEventId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."calendarEvent"(id) ON DELETE CASCADE;


--
-- Name: agentChatThread FK_256a1bec7a0b4ef293d5424c6c0; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThread"
    ADD CONSTRAINT "FK_256a1bec7a0b4ef293d5424c6c0" FOREIGN KEY ("assigneeId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE SET NULL;


--
-- Name: agentMessageReaction FK_27abe7185ced257ca8b54b07fe8; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentMessageReaction"
    ADD CONSTRAINT "FK_27abe7185ced257ca8b54b07fe8" FOREIGN KEY ("workspaceMemberId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE CASCADE;


--
-- Name: agentMessagePart FK_2aff9daad5cc3b5e15ca7173342; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentMessagePart"
    ADD CONSTRAINT "FK_2aff9daad5cc3b5e15ca7173342" FOREIGN KEY ("messageId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."agentMessage"(id) ON DELETE CASCADE;


--
-- Name: timelineActivity FK_393340ea834ab8660b51d482f29; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_393340ea834ab8660b51d482f29" FOREIGN KEY ("targetDashboardId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.dashboard(id) ON DELETE SET NULL;


--
-- Name: agentTurn FK_3be906dca9d5b50fbfe40e33f07; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentTurn"
    ADD CONSTRAINT "FK_3be906dca9d5b50fbfe40e33f07" FOREIGN KEY ("threadId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."agentChatThread"(id) ON DELETE CASCADE;


--
-- Name: timelineActivity FK_3c53994e4a67712996d6a9cbf8d; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_3c53994e4a67712996d6a9cbf8d" FOREIGN KEY ("targetWorkflowRunId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workflowRun"(id) ON DELETE CASCADE;


--
-- Name: agentMessage FK_4c31daa882e3130534995bf90ca; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentMessage"
    ADD CONSTRAINT "FK_4c31daa882e3130534995bf90ca" FOREIGN KEY ("threadId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."agentChatThread"(id) ON DELETE CASCADE;


--
-- Name: timelineActivity FK_4d266cbeff57096026be4b97ee3; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_4d266cbeff57096026be4b97ee3" FOREIGN KEY ("targetTaskId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.task(id) ON DELETE SET NULL;


--
-- Name: agentChatThreadParticipant FK_4e72c75eb6e4d7bd139b9d9a59b; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadParticipant"
    ADD CONSTRAINT "FK_4e72c75eb6e4d7bd139b9d9a59b" FOREIGN KEY ("threadId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."agentChatThread"(id) ON DELETE CASCADE;


--
-- Name: message FK_51fa5df1094bfdbf99ea91a72fa; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.message
    ADD CONSTRAINT "FK_51fa5df1094bfdbf99ea91a72fa" FOREIGN KEY ("messageThreadId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageThread"(id) ON DELETE CASCADE;


--
-- Name: noteTarget FK_52050fd9cf6b7ba9982ef9d867a; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."noteTarget"
    ADD CONSTRAINT "FK_52050fd9cf6b7ba9982ef9d867a" FOREIGN KEY ("noteId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.note(id) ON DELETE CASCADE;


--
-- Name: messageThreadTarget FK_56c24ab73a8903f72a7ab4b6824; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget"
    ADD CONSTRAINT "FK_56c24ab73a8903f72a7ab4b6824" FOREIGN KEY ("targetPersonId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE CASCADE;


--
-- Name: attachment FK_59071b50b641a16741369f61f1a; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.attachment
    ADD CONSTRAINT "FK_59071b50b641a16741369f61f1a" FOREIGN KEY ("targetNoteId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.note(id) ON DELETE SET NULL;


--
-- Name: agentMessageReaction FK_59e9b769814f37030e0490b62e3; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentMessageReaction"
    ADD CONSTRAINT "FK_59e9b769814f37030e0490b62e3" FOREIGN KEY ("messageId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."agentMessage"(id) ON DELETE CASCADE;


--
-- Name: opportunity FK_5a104112b21bbebb37fca93a548; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.opportunity
    ADD CONSTRAINT "FK_5a104112b21bbebb37fca93a548" FOREIGN KEY ("companyId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.company(id) ON DELETE SET NULL;


--
-- Name: noteTarget FK_5afbc6b8d586b33b1985c5ca1ff; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."noteTarget"
    ADD CONSTRAINT "FK_5afbc6b8d586b33b1985c5ca1ff" FOREIGN KEY ("targetPersonId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE CASCADE;


--
-- Name: calendarEventTarget FK_5f6255902edcffc1d11aa3a1e23; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget"
    ADD CONSTRAINT "FK_5f6255902edcffc1d11aa3a1e23" FOREIGN KEY ("targetOpportunityId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.opportunity(id) ON DELETE CASCADE;


--
-- Name: messageThreadTarget FK_61beeb39fe0296fb56068b1fcfa; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget"
    ADD CONSTRAINT "FK_61beeb39fe0296fb56068b1fcfa" FOREIGN KEY ("targetOpportunityId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.opportunity(id) ON DELETE CASCADE;


--
-- Name: messageListMember FK_62be63807685ee75c30227fd92f; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageListMember"
    ADD CONSTRAINT "FK_62be63807685ee75c30227fd92f" FOREIGN KEY ("listId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageList"(id) ON DELETE CASCADE;


--
-- Name: noteTarget FK_659eca453a19006d9f12e76e646; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."noteTarget"
    ADD CONSTRAINT "FK_659eca453a19006d9f12e76e646" FOREIGN KEY ("targetOpportunityId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.opportunity(id) ON DELETE CASCADE;


--
-- Name: messageParticipant FK_670c8b151ad702613fce89b3662; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageParticipant"
    ADD CONSTRAINT "FK_670c8b151ad702613fce89b3662" FOREIGN KEY ("messageId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.message(id) ON DELETE CASCADE;


--
-- Name: opportunity FK_67371a1e3fd5bac524f7e914bad; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.opportunity
    ADD CONSTRAINT "FK_67371a1e3fd5bac524f7e914bad" FOREIGN KEY ("ownerId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE SET NULL;


--
-- Name: task FK_7384988f7eeb777e44802a0baca; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.task
    ADD CONSTRAINT "FK_7384988f7eeb777e44802a0baca" FOREIGN KEY ("assigneeId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE SET NULL;


--
-- Name: attachment FK_73f3acb2a2eebe7834a9918116d; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.attachment
    ADD CONSTRAINT "FK_73f3acb2a2eebe7834a9918116d" FOREIGN KEY ("targetPersonId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE CASCADE;


--
-- Name: timelineActivity FK_7460f3992bc8d73165b3b2874e6; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_7460f3992bc8d73165b3b2874e6" FOREIGN KEY ("targetCompanyId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.company(id) ON DELETE CASCADE;


--
-- Name: callRecording FK_77bacb61539822c0763f3382c14; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."callRecording"
    ADD CONSTRAINT "FK_77bacb61539822c0763f3382c14" FOREIGN KEY ("calendarEventId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."calendarEvent"(id) ON DELETE SET NULL;


--
-- Name: calendarEventParticipant FK_78e8a7e863f3d614d3d30034479; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant"
    ADD CONSTRAINT "FK_78e8a7e863f3d614d3d30034479" FOREIGN KEY ("personId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE SET NULL;


--
-- Name: taskTarget FK_7b5ee65641d4f0b15cdde24aef5; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."taskTarget"
    ADD CONSTRAINT "FK_7b5ee65641d4f0b15cdde24aef5" FOREIGN KEY ("targetOpportunityId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.opportunity(id) ON DELETE CASCADE;


--
-- Name: taskTarget FK_7e4d4dfe6a4af120a7ece98d77d; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."taskTarget"
    ADD CONSTRAINT "FK_7e4d4dfe6a4af120a7ece98d77d" FOREIGN KEY ("targetCompanyId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.company(id) ON DELETE CASCADE;


--
-- Name: messageParticipant FK_840b9ccea0055abb8b3e980638c; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageParticipant"
    ADD CONSTRAINT "FK_840b9ccea0055abb8b3e980638c" FOREIGN KEY ("workspaceMemberId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE SET NULL;


--
-- Name: timelineActivity FK_875809b410c253073b5aa812c9a; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_875809b410c253073b5aa812c9a" FOREIGN KEY ("targetPersonId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE CASCADE;


--
-- Name: agentMessage FK_87dbab10ac94d9a091f8efaa67b; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentMessage"
    ADD CONSTRAINT "FK_87dbab10ac94d9a091f8efaa67b" FOREIGN KEY ("turnId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."agentTurn"(id) ON DELETE CASCADE;


--
-- Name: agentChatThreadTarget FK_9141b0e1cc92a0b5a55b7b3dd58; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget"
    ADD CONSTRAINT "FK_9141b0e1cc92a0b5a55b7b3dd58" FOREIGN KEY ("targetCompanyId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.company(id) ON DELETE CASCADE;


--
-- Name: messageChannelMessageAssociation FK_9b53fe617a3a4c0fa231703a64f; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation"
    ADD CONSTRAINT "FK_9b53fe617a3a4c0fa231703a64f" FOREIGN KEY ("messageId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.message(id) ON DELETE CASCADE;


--
-- Name: messageCampaign FK_9bac7666349e430e00b5d6a6d61; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageCampaign"
    ADD CONSTRAINT "FK_9bac7666349e430e00b5d6a6d61" FOREIGN KEY ("listId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageList"(id) ON DELETE SET NULL;


--
-- Name: agentChatThread FK_a16ebdffd99d29d0654fe67807f; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThread"
    ADD CONSTRAINT "FK_a16ebdffd99d29d0654fe67807f" FOREIGN KEY ("workspaceMemberId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE CASCADE;


--
-- Name: timelineActivity FK_a673e094d8178750dc4bba893cc; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_a673e094d8178750dc4bba893cc" FOREIGN KEY ("targetMessageListId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageList"(id) ON DELETE SET NULL;


--
-- Name: calendarEventTarget FK_a9cec8134b8cbefb3b5612210a5; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget"
    ADD CONSTRAINT "FK_a9cec8134b8cbefb3b5612210a5" FOREIGN KEY ("targetCompanyId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.company(id) ON DELETE CASCADE;


--
-- Name: timelineActivity FK_ab1ef862f90b03052b7c697c7a5; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_ab1ef862f90b03052b7c697c7a5" FOREIGN KEY ("targetMessageCampaignId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageCampaign"(id) ON DELETE SET NULL;


--
-- Name: company FK_abe91dff1e48d77edca339f387c; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.company
    ADD CONSTRAINT "FK_abe91dff1e48d77edca339f387c" FOREIGN KEY ("accountOwnerId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE SET NULL;


--
-- Name: noteTarget FK_b10df13291b5386b984298bdf20; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."noteTarget"
    ADD CONSTRAINT "FK_b10df13291b5386b984298bdf20" FOREIGN KEY ("targetCompanyId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.company(id) ON DELETE CASCADE;


--
-- Name: agentMessage FK_b4dc2405a344ec5fc1ce7dcfd7c; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentMessage"
    ADD CONSTRAINT "FK_b4dc2405a344ec5fc1ce7dcfd7c" FOREIGN KEY ("senderWorkspaceMemberId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE SET NULL;


--
-- Name: messageChannelMessageAssociationMessageFolder FK_b5510ced6b48600c5a1ab14ec42; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociationMessageFolder"
    ADD CONSTRAINT "FK_b5510ced6b48600c5a1ab14ec42" FOREIGN KEY ("messageChannelMessageAssociationId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation"(id) ON DELETE CASCADE;


--
-- Name: attachment FK_b7c6ea80823b826f32fcea66890; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.attachment
    ADD CONSTRAINT "FK_b7c6ea80823b826f32fcea66890" FOREIGN KEY ("targetCompanyId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.company(id) ON DELETE CASCADE;


--
-- Name: timelineActivity FK_b8af01591f389f596f7be54b13d; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."timelineActivity"
    ADD CONSTRAINT "FK_b8af01591f389f596f7be54b13d" FOREIGN KEY ("workspaceMemberId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE CASCADE;


--
-- Name: blocklist FK_bbaebad9d11e773689cb3cdf997; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.blocklist
    ADD CONSTRAINT "FK_bbaebad9d11e773689cb3cdf997" FOREIGN KEY ("workspaceMemberId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE CASCADE;


--
-- Name: messageThreadTarget FK_bfa1faa6fe9a01833c12912429c; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget"
    ADD CONSTRAINT "FK_bfa1faa6fe9a01833c12912429c" FOREIGN KEY ("targetCompanyId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.company(id) ON DELETE CASCADE;


--
-- Name: agentChatThreadTarget FK_c8cf757f0fa3d7dd3f8d45cbaca; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget"
    ADD CONSTRAINT "FK_c8cf757f0fa3d7dd3f8d45cbaca" FOREIGN KEY ("threadId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."agentChatThread"(id) ON DELETE CASCADE;


--
-- Name: taskTarget FK_c95f3cce2b95309005a9d1ed824; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."taskTarget"
    ADD CONSTRAINT "FK_c95f3cce2b95309005a9d1ed824" FOREIGN KEY ("taskId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.task(id) ON DELETE CASCADE;


--
-- Name: messageParticipant FK_cbfaa3bb38cced64ea9576016c2; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageParticipant"
    ADD CONSTRAINT "FK_cbfaa3bb38cced64ea9576016c2" FOREIGN KEY ("personId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE SET NULL;


--
-- Name: calendarEventParticipant FK_cd980caeb07e091a71780e5a367; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEventParticipant"
    ADD CONSTRAINT "FK_cd980caeb07e091a71780e5a367" FOREIGN KEY ("workspaceMemberId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."workspaceMember"(id) ON DELETE SET NULL;


--
-- Name: attachment FK_ce25e08f787c3eeabf25bbf1084; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.attachment
    ADD CONSTRAINT "FK_ce25e08f787c3eeabf25bbf1084" FOREIGN KEY ("targetAgentChatThreadId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."agentChatThread"(id) ON DELETE CASCADE;


--
-- Name: opportunity FK_ce5aa294b17fe23a0b19267f1e1; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.opportunity
    ADD CONSTRAINT "FK_ce5aa294b17fe23a0b19267f1e1" FOREIGN KEY ("pointOfContactId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE SET NULL;


--
-- Name: messageThreadTarget FK_e3164a618efda5efba17de09ad8; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageThreadTarget"
    ADD CONSTRAINT "FK_e3164a618efda5efba17de09ad8" FOREIGN KEY ("messageThreadId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageThread"(id) ON DELETE CASCADE;


--
-- Name: agentChatThreadTarget FK_e3b662431e7c409c46e8e454e59; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."agentChatThreadTarget"
    ADD CONSTRAINT "FK_e3b662431e7c409c46e8e454e59" FOREIGN KEY ("targetPersonId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.person(id) ON DELETE CASCADE;


--
-- Name: message FK_e6f710e358c428f95094a4adcaf; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.message
    ADD CONSTRAINT "FK_e6f710e358c428f95094a4adcaf" FOREIGN KEY ("messageCampaignId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageCampaign"(id) ON DELETE SET NULL;


--
-- Name: attachment FK_e7ae79d27ad97d13b4b7638928c; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.attachment
    ADD CONSTRAINT "FK_e7ae79d27ad97d13b4b7638928c" FOREIGN KEY ("targetTaskId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.task(id) ON DELETE SET NULL;


--
-- Name: messageChannelMessageAssociation FK_ed17eebc0ece37ca05951cefb30; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."messageChannelMessageAssociation"
    ADD CONSTRAINT "FK_ed17eebc0ece37ca05951cefb30" FOREIGN KEY ("messageThreadId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."messageThread"(id) ON DELETE CASCADE;


--
-- Name: person FK_ee066ddacfce46c9a7cb90edd1a; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.person
    ADD CONSTRAINT "FK_ee066ddacfce46c9a7cb90edd1a" FOREIGN KEY ("companyId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.company(id) ON DELETE SET NULL;


--
-- Name: calendarEventTarget FK_f40e8a55ddff2b80d781cbd9bbb; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0."calendarEventTarget"
    ADD CONSTRAINT "FK_f40e8a55ddff2b80d781cbd9bbb" FOREIGN KEY ("calendarEventId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0."calendarEvent"(id) ON DELETE CASCADE;


--
-- Name: attachment FK_f46951be10839ac85e570507b35; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.attachment
    ADD CONSTRAINT "FK_f46951be10839ac85e570507b35" FOREIGN KEY ("targetOpportunityId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.opportunity(id) ON DELETE CASCADE;


--
-- Name: attachment FK_fff5fb78a7df5db6586ac686c47; Type: FK CONSTRAINT; Schema: workspace_6n57znb4w89f8r7xudebmf0; Owner: -
--

ALTER TABLE ONLY workspace_6n57znb4w89f8r7xudebmf0.attachment
    ADD CONSTRAINT "FK_fff5fb78a7df5db6586ac686c47" FOREIGN KEY ("targetDashboardId") REFERENCES workspace_6n57znb4w89f8r7xudebmf0.dashboard(id) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--

\unrestrict oBWiH0mrTa4WiBcUuNf82QTO3l2MXJiewyaCWDdlVwEFyWeV2IlDZ80tz9Cyfr8

