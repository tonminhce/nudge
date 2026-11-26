use axum::extract::{Path, Query, State};
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::Form;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;
use chrono::{DateTime, Utc};
use flate2::write::DeflateEncoder;
use flate2::Compression;
use rand::RngCore;
use std::collections::HashMap;
use std::io::Write;
use uuid::Uuid;

#[cfg(test)]
use flate2::read::DeflateDecoder;

#[cfg(test)]
use std::io::Read;

use super::super::error::{AuthError, AuthResult};
use super::flow::{self, idp_not_found_error};
use super::state_store::{save_state, take_state, SamlRelayStatePayload};
use super::{optional_query_string, redirect_response};
use crate::state::AppState;

const SAML_BEARER_CONFIRMATION_METHOD: &str = "urn:oasis:names:tc:SAML:2.0:cm:bearer";
const METADATA_CONTENT_TYPE: &str = "application/samlmetadata+xml";

#[derive(Debug, Clone)]
pub struct XmlElement {
    name: String,
    attributes: Vec<(String, String)>,
    text: String,
    children: Vec<XmlElement>,
}

impl XmlElement {
    fn local_name(&self) -> &str {
        match self.name.rsplit_once(':') {
            Some((_, local)) => local,
            None => &self.name,
        }
    }

    fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| {
                key == name || key.rsplit_once(':').is_some_and(|(_, local)| local == name)
            })
            .map(|(_, value)| value.as_str())
    }

    fn find_all(&self, local_name: &str) -> Vec<&XmlElement> {
        let mut found = Vec::new();

        if self.local_name() == local_name {
            found.push(self);
        }

        for child in &self.children {
            found.extend(child.find_all(local_name));
        }

        found
    }

    fn find_first(&self, local_name: &str) -> Option<&XmlElement> {
        self.find_all(local_name).into_iter().next()
    }
}

fn parse_xml(raw: &str) -> AuthResult<XmlElement> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(raw);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<XmlElement> = Vec::new();
    let mut root: Option<XmlElement> = None;

    loop {
        let event = reader
            .read_event()
            .map_err(|error| saml_parse_error(error.to_string()))?;

        match event {
            Event::Start(start) => {
                stack.push(read_element(&start)?);
            }
            Event::Empty(empty) => {
                let element = read_element(&empty)?;

                match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => root = Some(element),
                }
            }
            Event::Text(text) => {
                let content = text
                    .unescape()
                    .map_err(|error| saml_parse_error(error.to_string()))?;

                if let Some(top) = stack.last_mut() {
                    top.text.push_str(content.trim());
                }
            }
            Event::End(_) => match stack.pop() {
                Some(element) => match stack.last_mut() {
                    Some(parent) => parent.children.push(element),
                    None => root = Some(element),
                },
                None => return Err(saml_parse_error("unbalanced SAML response".to_string())),
            },
            Event::Eof => break,
            _ => {}
        }
    }

    root.ok_or_else(|| saml_parse_error("empty SAML response".to_string()))
}

fn read_element(element: &quick_xml::events::BytesStart<'_>) -> AuthResult<XmlElement> {
    let name = String::from_utf8_lossy(element.name().as_ref()).to_string();

    let mut attributes = Vec::new();

    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| saml_parse_error(error.to_string()))?;

        let key = String::from_utf8_lossy(attribute.key.as_ref()).to_string();
        let value = String::from_utf8_lossy(&attribute.value).to_string();

        attributes.push((key, value));
    }

    Ok(XmlElement {
        name,
        attributes,
        text: String::new(),
        children: Vec::new(),
    })
}

fn saml_parse_error(message: String) -> AuthError {
    AuthError::invalid_input(format!("Invalid SAML response: {message}"))
}

fn sso_auth_failed_error(message: &str) -> AuthError {
    AuthError::new(403, "SSO_AUTH_FAILED", message.to_string())
}

#[derive(Debug)]
pub struct ParsedAssertion {
    pub assertion_id: String,
    pub valid_until: DateTime<Utc>,
    pub email: Option<String>,
}

pub fn decode_saml_response(raw: &str) -> AuthResult<String> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(saml_parse_error(
            "the SAMLResponse is not valid base64".to_string(),
        ));
    }

    let bytes = STANDARD
        .decode(trimmed)
        .or_else(|_| STANDARD_NO_PAD.decode(trimmed))
        .map_err(|_| saml_parse_error("the SAMLResponse is not valid base64".to_string()))?;

    String::from_utf8(bytes)
        .map_err(|_| saml_parse_error("the SAMLResponse is not valid UTF-8".to_string()))
}

fn parse_saml_date(raw: Option<&str>) -> Option<DateTime<Utc>> {
    raw.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|parsed| parsed.with_timezone(&Utc))
}

pub fn extract_assertion(document: &XmlElement) -> AuthResult<ParsedAssertion> {
    let assertion = document
        .find_first("Assertion")
        .ok_or_else(|| saml_parse_error("no SAML assertion found".to_string()))?;

    let assertion_id = assertion
        .attribute("ID")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| sso_auth_failed_error("SAML assertion has no ID or bearer NotOnOrAfter"))?
        .to_string();

    let mut bearer_not_on_or_after: Vec<DateTime<Utc>> = Vec::new();

    for subject in assertion.find_all("SubjectConfirmation") {
        if subject.attribute("Method") != Some(SAML_BEARER_CONFIRMATION_METHOD) {
            continue;
        }

        for data in subject.find_all("SubjectConfirmationData") {
            if let Some(not_on_or_after) = parse_saml_date(data.attribute("NotOnOrAfter")) {
                bearer_not_on_or_after.push(not_on_or_after);
            }
        }
    }

    let valid_until = bearer_not_on_or_after
        .into_iter()
        .max()
        .ok_or_else(|| sso_auth_failed_error("SAML assertion has no ID or bearer NotOnOrAfter"))?;

    if let Some(conditions) = assertion.find_first("Conditions") {
        if let Some(not_before) = parse_saml_date(conditions.attribute("NotBefore")) {
            if Utc::now() < not_before {
                return Err(sso_auth_failed_error("SAML assertion is not yet valid"));
            }
        }

        if let Some(not_on_or_after) = parse_saml_date(conditions.attribute("NotOnOrAfter")) {
            if Utc::now() >= not_on_or_after {
                return Err(sso_auth_failed_error("SAML assertion has expired"));
            }
        }
    }

    let email = extract_assertion_email(assertion);

    Ok(ParsedAssertion {
        assertion_id,
        valid_until,
        email,
    })
}

fn extract_assertion_email(assertion: &XmlElement) -> Option<String> {
    for attribute in assertion.find_all("Attribute") {
        let name = attribute.attribute("Name").unwrap_or_default();

        if name == "email" || name == "mail" {
            if let Some(value) = attribute.find_first("AttributeValue") {
                let text = value.text.trim();

                if !text.is_empty() {
                    return Some(text.to_string());
                }
            }
        }
    }

    let name_id = assertion
        .find_first("NameID")
        .map(|element| element.text.trim().to_string())
        .filter(|text| !text.is_empty());

    name_id
}

pub fn build_sp_metadata_xml(identity_provider_id: &str, server_url: &str) -> String {
    let issuer = build_entity_id(server_url, identity_provider_id);
    let callback_url = build_callback_url(server_url, identity_provider_id);

    format!(
        "<md:EntityDescriptor xmlns:md=\"urn:oasis:names:tc:SAML:2.0:metadata\" entityID=\"{}\">\
<md:SPSSODescriptor WantAssertionsSigned=\"true\" protocolSupportEnumeration=\"urn:oasis:names:tc:SAML:2.0:protocol\">\
<md:NameIDFormat Format=\"urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress\"/>\
<md:AssertionConsumerService Binding=\"urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST\" Location=\"{}\" index=\"0\"/>\
</md:SPSSODescriptor>\
</md:EntityDescriptor>",
        super::xml_escape(&issuer),
        super::xml_escape(&callback_url),
    )
}

fn build_entity_id(server_url: &str, identity_provider_id: &str) -> String {
    format!(
        "{}/auth/saml/login/{}",
        server_url.trim_end_matches('/'),
        identity_provider_id
    )
}

fn build_callback_url(server_url: &str, identity_provider_id: &str) -> String {
    format!(
        "{}/auth/saml/callback/{}",
        server_url.trim_end_matches('/'),
        identity_provider_id
    )
}

pub fn deflate_and_encode(raw: &str) -> AuthResult<String> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());

    encoder
        .write_all(raw.as_bytes())
        .map_err(|error| AuthError::internal(error.to_string()))?;

    let deflated = encoder
        .finish()
        .map_err(|error| AuthError::internal(error.to_string()))?;

    Ok(STANDARD.encode(deflated))
}

#[cfg(test)]
pub fn decode_and_inflate(raw: &str) -> AuthResult<String> {
    let bytes = STANDARD
        .decode(raw.trim())
        .map_err(|error| AuthError::internal(error.to_string()))?;

    let mut decoder = DeflateDecoder::new(&bytes[..]);

    let mut inflated = String::new();

    decoder
        .read_to_string(&mut inflated)
        .map_err(|error| AuthError::internal(error.to_string()))?;

    Ok(inflated)
}

pub fn build_authn_request_xml(issuer_entity_id: &str, destination: &str) -> AuthResult<String> {
    let mut id_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut id_bytes);

    let request_id = format!("_{}", hex::encode(id_bytes));
    let issue_instant = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    Ok(format!(
        "<samlp:AuthnRequest xmlns:samlp=\"urn:oasis:names:tc:SAML:2.0:protocol\" xmlns:saml=\"urn:oasis:names:tc:SAML:2.0:assertion\" ID=\"{}\" Version=\"2.0\" IssueInstant=\"{}\" Destination=\"{}\">\
<saml:Issuer>{}</saml:Issuer>\
</samlp:AuthnRequest>",
        request_id,
        issue_instant,
        super::xml_escape(destination),
        super::xml_escape(issuer_entity_id),
    ))
}

struct SamlIdentityProviderRow {
    sso_url: String,
}

async fn load_saml_identity_provider(
    state: &AppState,
    identity_provider_id: &str,
) -> AuthResult<SamlIdentityProviderRow> {
    let id = Uuid::parse_str(identity_provider_id).map_err(|_| idp_not_found_error())?;

    let row: Option<(Option<String>, String)> = sqlx::query_as(
        "SELECT \"ssoURL\", type FROM core.\"workspaceSSOIdentityProvider\" WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.core_pool)
    .await?;

    let (sso_url, idp_type) = row.ok_or_else(idp_not_found_error)?;

    if idp_type != "SAML" {
        return Err(idp_not_found_error());
    }

    Ok(SamlIdentityProviderRow {
        sso_url: sso_url.ok_or_else(idp_not_found_error)?,
    })
}

#[derive(serde::Deserialize)]
pub struct SamlCallbackForm {
    #[serde(rename = "SAMLResponse")]
    pub saml_response: Option<String>,
    #[serde(rename = "RelayState")]
    pub relay_state: Option<String>,
}

pub async fn metadata(
    State(state): State<AppState>,
    Path(identity_provider_id): Path<String>,
) -> AuthResult<Response> {
    let xml = build_sp_metadata_xml(
        &identity_provider_id,
        state.config.server_url.trim_end_matches('/'),
    );

    let mut response = (axum::http::StatusCode::OK, xml).into_response();

    response.headers_mut().insert(
        CONTENT_TYPE,
        axum::http::HeaderValue::from_static(METADATA_CONTENT_TYPE),
    );

    Ok(response)
}

pub async fn login(
    State(state): State<AppState>,
    Path(identity_provider_id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> AuthResult<Response> {
    let result = login_redirect(&state, &identity_provider_id, &query).await;

    match result {
        Ok(redirect) => Ok(redirect),
        Err(error) => Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            &error.messages.join(", "),
        ))),
    }
}

async fn login_redirect(
    state: &AppState,
    identity_provider_id: &str,
    query: &HashMap<String, String>,
) -> AuthResult<Response> {
    let identity_provider = load_saml_identity_provider(state, identity_provider_id).await?;

    let entity_id = build_entity_id(
        state.config.server_url.trim_end_matches('/'),
        identity_provider_id,
    );

    let request_xml = build_authn_request_xml(&entity_id, &identity_provider.sso_url)?;

    let saml_request = deflate_and_encode(&request_xml)?;

    let mut redirect_url = format!(
        "{}?SAMLRequest={}",
        identity_provider.sso_url,
        super::percent_encode(&saml_request)
    );

    let relay_state = SamlRelayStatePayload {
        identity_provider_id: identity_provider_id.to_string(),
        workspace_invite_hash: optional_query_string(
            query.get("workspaceInviteHash").map(String::as_str),
        ),
        workspace_personal_invite_token: optional_query_string(
            query
                .get("workspacePersonalInviteToken")
                .map(String::as_str),
        ),
    };

    let has_relay_payload = relay_state.workspace_invite_hash.is_some()
        || relay_state.workspace_personal_invite_token.is_some();

    if has_relay_payload {
        let relay_key = save_state(state, &relay_state).await?;

        redirect_url.push_str(&format!(
            "&RelayState={}",
            super::percent_encode(&relay_key)
        ));
    }

    Ok(redirect_response(&redirect_url))
}

pub async fn callback(
    State(state): State<AppState>,
    Path(identity_provider_id): Path<String>,
    Form(form): Form<SamlCallbackForm>,
) -> AuthResult<Response> {
    if !super::config::saml_accepts_unverified_assertions() {
        return Err(AuthError::new(
            403,
            "SSO_AUTH_FAILED",
            "SAML signature verification is not available in this deployment. \
             Set AUTH_SAML_ACCEPT_UNVERIFIED_ASSERTIONS=true to enable SAML sign-in \
             without signature verification (not recommended).",
        ));
    }

    let result = callback_redirect(&state, &identity_provider_id, &form).await;

    match result {
        Ok(redirect) => Ok(redirect),
        Err(error) => Ok(redirect_response(&flow::error_redirect_url(
            &state.config,
            &error.messages.join(", "),
        ))),
    }
}

async fn callback_redirect(
    state: &AppState,
    identity_provider_id: &str,
    form: &SamlCallbackForm,
) -> AuthResult<Response> {
    let Some(raw_response) = form.saml_response.as_deref().filter(|raw| !raw.is_empty()) else {
        return Err(AuthError::invalid_input("Profile must be provided"));
    };

    let _ = load_saml_identity_provider(state, identity_provider_id).await?;

    let document = parse_xml(&decode_saml_response(raw_response)?)?;

    let assertion = extract_assertion(&document)?;

    if Utc::now() >= assertion.valid_until {
        return Err(sso_auth_failed_error("SAML assertion has expired"));
    }

    consume_assertion_first_use(
        state,
        identity_provider_id,
        &assertion.assertion_id,
        assertion.valid_until,
    )
    .await?;

    let Some(email) = assertion.email.as_deref() else {
        return Err(AuthError::invalid_input("Invalid email"));
    };

    let normalized = super::super::service::normalize_email(email)
        .map_err(|_| AuthError::invalid_input("Invalid email"))?;

    let mut workspace_invite_hash = None;
    let mut workspace_personal_invite_token = None;

    if let Some(relay_key) = form.relay_state.as_deref().filter(|raw| !raw.is_empty()) {
        let relay: SamlRelayStatePayload = take_state(state, relay_key)
            .await?
            .ok_or_else(|| AuthError::invalid_input("Invalid state"))?;

        if relay.identity_provider_id != identity_provider_id {
            return Err(AuthError::invalid_input("Invalid state"));
        }

        workspace_invite_hash = relay.workspace_invite_hash;
        workspace_personal_invite_token = relay.workspace_personal_invite_token;
    }

    let redirect = flow::sso_idp_callback_redirect_url(
        state,
        identity_provider_id,
        &normalized,
        workspace_invite_hash.as_deref(),
        workspace_personal_invite_token.as_deref(),
    )
    .await?;

    Ok(redirect_response(&redirect))
}

async fn consume_assertion_first_use(
    state: &AppState,
    identity_provider_id: &str,
    assertion_id: &str,
    valid_until: DateTime<Utc>,
) -> AuthResult<()> {
    let ttl = (valid_until - Utc::now()).num_seconds().max(1);

    let mut connection = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    let acquired: Option<String> = redis::cmd("SET")
        .arg(format!(
            "saml-assertion:consumed:{identity_provider_id}:{assertion_id}"
        ))
        .arg("1")
        .arg("NX")
        .arg("EX")
        .arg(ttl)
        .query_async(&mut connection)
        .await
        .map_err(|error| AuthError::internal(error.to_string()))?;

    if acquired.is_none() {
        return Err(sso_auth_failed_error(
            "SAML assertion has already been used",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE_FIXTURE: &str = r#"<samlp:Response xmlns:samlp="urn:oasis:names:tc:SAML:2.0:protocol" ID="_resp" Version="2.0">
      <saml:Assertion xmlns:saml="urn:oasis:names:tc:SAML:2.0:assertion" ID="_assertion1" Version="2.0">
        <saml:Subject>
          <saml:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">fallback@example.com</saml:NameID>
          <saml:SubjectConfirmation Method="urn:oasis:names:tc:SAML:2.0:cm:bearer">
            <saml:SubjectConfirmationData NotOnOrAfter="2030-01-01T00:00:00Z" Recipient="https://server/auth/saml/callback/idp-1"/>
          </saml:SubjectConfirmation>
        </saml:Subject>
        <saml:Conditions NotBefore="2020-01-01T00:00:00Z" NotOnOrAfter="2030-01-01T00:00:00Z">
          <saml:AudienceRestriction><saml:Audience>https://server/auth/saml/login/idp-1</saml:Audience></saml:AudienceRestriction>
        </saml:Conditions>
        <saml:AttributeStatement>
          <saml:Attribute Name="mail"><saml:AttributeValue>jane@example.com</saml:AttributeValue></saml:Attribute>
        </saml:AttributeStatement>
      </saml:Assertion>
    </samlp:Response>"#;

    #[test]
    fn sp_metadata_carries_the_entity_id_and_callback_locations() {
        let xml = build_sp_metadata_xml("idp-1", "http://localhost:3000");

        assert!(xml.contains("entityID=\"http://localhost:3000/auth/saml/login/idp-1\""));
        assert!(xml.contains("WantAssertionsSigned=\"true\""));
        assert!(xml.contains("Location=\"http://localhost:3000/auth/saml/callback/idp-1\""));
    }

    #[test]
    fn authn_requests_round_trip_through_the_redirect_binding() {
        let xml = build_authn_request_xml(
            "http://localhost:3000/auth/saml/login/idp-1",
            "https://idp.example.com/sso",
        )
        .unwrap();

        assert!(
            xml.contains("<saml:Issuer>http://localhost:3000/auth/saml/login/idp-1</saml:Issuer>")
        );
        assert!(xml.contains("Destination=\"https://idp.example.com/sso\""));
        assert!(xml.contains("Version=\"2.0\""));
        assert!(xml.contains("ID=\"_"));

        let encoded = deflate_and_encode(&xml).unwrap();
        let decoded = decode_and_inflate(&encoded).unwrap();

        assert_eq!(decoded, xml);
    }

    #[test]
    fn assertion_fixture_yields_id_bearer_window_and_email() {
        let document = parse_xml(RESPONSE_FIXTURE).unwrap();
        let assertion = extract_assertion(&document).unwrap();

        assert_eq!(assertion.assertion_id, "_assertion1");
        assert_eq!(
            assertion.valid_until,
            DateTime::parse_from_rfc3339("2030-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc)
        );
        assert_eq!(assertion.email.as_deref(), Some("jane@example.com"));
    }

    #[test]
    fn email_falls_back_to_name_id_when_no_attributes_match() {
        let fixture = RESPONSE_FIXTURE
            .replace(
                r#"<saml:AttributeStatement>
          <saml:Attribute Name="mail"><saml:AttributeValue>jane@example.com</saml:AttributeValue></saml:Attribute>
        </saml:AttributeStatement>"#,
                "",
            );

        let document = parse_xml(&fixture).unwrap();
        let assertion = extract_assertion(&document).unwrap();

        assert_eq!(assertion.email.as_deref(), Some("fallback@example.com"));
    }

    #[test]
    fn assertions_without_a_bearer_confirmation_are_rejected() {
        let fixture = RESPONSE_FIXTURE.replace(
            "urn:oasis:names:tc:SAML:2.0:cm:bearer",
            "urn:oasis:names:tc:SAML:2.0:cm:holder-of-key",
        );

        let document = parse_xml(&fixture).unwrap();

        let error = extract_assertion(&document).unwrap_err();

        assert_eq!(error.code, "SSO_AUTH_FAILED");
        assert_eq!(
            error.messages.join(", "),
            "SAML assertion has no ID or bearer NotOnOrAfter"
        );
    }

    #[test]
    fn assertions_without_an_id_are_rejected() {
        let fixture = RESPONSE_FIXTURE.replace("ID=\"_assertion1\"", "");

        let document = parse_xml(&fixture).unwrap();
        let error = extract_assertion(&document).unwrap_err();

        assert_eq!(error.code, "SSO_AUTH_FAILED");
    }

    #[test]
    fn expired_conditions_are_rejected() {
        let fixture = RESPONSE_FIXTURE.replace(
            "NotBefore=\"2020-01-01T00:00:00Z\" NotOnOrAfter=\"2030-01-01T00:00:00Z\"",
            "NotBefore=\"2020-01-01T00:00:00Z\" NotOnOrAfter=\"2021-01-01T00:00:00Z\"",
        );

        let document = parse_xml(&fixture).unwrap();
        let error = extract_assertion(&document).unwrap_err();

        assert_eq!(error.messages.join(", "), "SAML assertion has expired");
    }

    #[test]
    fn future_conditions_are_rejected() {
        let fixture = RESPONSE_FIXTURE.replace(
            "NotBefore=\"2020-01-01T00:00:00Z\"",
            "NotBefore=\"2100-01-01T00:00:00Z\"",
        );

        let document = parse_xml(&fixture).unwrap();
        let error = extract_assertion(&document).unwrap_err();

        assert_eq!(error.messages.join(", "), "SAML assertion is not yet valid");
    }

    #[test]
    fn invalid_base64_responses_are_rejected() {
        assert!(decode_saml_response("not base64!!").is_err());
        assert!(decode_saml_response("").is_err());
    }

    #[test]
    fn replay_keys_use_the_upstream_namespace() {
        let idp = "11111111-2222-3333-4444-555555555555";
        let assertion_id = "_a1";

        assert_eq!(
            format!("saml-assertion:consumed:{idp}:{assertion_id}"),
            "saml-assertion:consumed:11111111-2222-3333-4444-555555555555:_a1"
        );
    }
}
