use front_core::client::BrowserGraphQLClient;
use front_core::client::Endpoint;
use front_core::client::GraphQLClient;
use front_core::client::RequestOptions;
use front_ui::components::{Toast, ToastTone, ToastViewport};
use front_ui::icons::Icon;
use front_ui::icons::IconSun;
use front_ui::tokens::Scheme;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use serde_json::json;

use crate::app::server_url;
use crate::app::write_storage;
use crate::app::AuthState;

const LOGO_MARK: &str = include_str!("../assets/nudge-mark.svg");

#[component]
pub fn ThemeToggle() -> impl IntoView {
    let theme = expect_context::<RwSignal<Scheme>>();
    view! {
        <button class="app-icon-button theme-toggle" aria-label="Toggle theme" on:click=move |_| {
            let next = match theme.get() {
                Scheme::Light => Scheme::Dark,
                Scheme::Dark => Scheme::Light,
            };
            theme.set(next);
            write_storage("nudge.theme", match next {
                Scheme::Light => "light",
                Scheme::Dark => "dark",
            });
        }>
            <IconSun size=18 />
        </button>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AuthStep {
    Email,
    Password,
}

#[component]
pub fn AuthPage() -> impl IntoView {
    let auth = expect_context::<AuthState>();
    let navigate = use_navigate();
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let step = RwSignal::new(AuthStep::Email);
    let user_exists = RwSignal::new(false);
    let password_visible = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    let notice = RwSignal::new(None::<String>);

    let submit = move || match step.get() {
        AuthStep::Email => {
            let email_value = email.get();
            if !email_value.trim().is_empty() {
                error.set(None);
                loading.set(true);
                leptos::task::spawn_local(async move {
                    match check_user(&email_value).await {
                        Ok(exists) => {
                            user_exists.set(exists);
                            password_visible.set(false);
                            step.set(AuthStep::Password);
                            loading.set(false);
                        }
                        Err(message) => {
                            error.set(Some(message));
                            loading.set(false);
                        }
                    }
                });
            }
        }
        AuthStep::Password => {
            let email_value = email.get();
            let password_value = password.get();
            let exists = user_exists.get();
            let auth = auth.clone();
            let navigate = navigate.clone();
            error.set(None);
            loading.set(true);
            leptos::task::spawn_local(async move {
                let result = if exists {
                    sign_up(&email_value, &password_value).await
                } else {
                    sign_in(&email_value, &password_value).await
                };
                match result {
                    Ok(session) => {
                        auth.token.set(Some(session.token.clone()));
                        auth.workspace_id.set(Some(session.workspace_id.clone()));
                        write_storage("nudge.token", &session.token);
                        write_storage("nudge.workspaceId", &session.workspace_id);
                        if let Ok(user) = fetch_current_user(&session.token).await {
                            auth.user.set(Some(user));
                        }
                        navigate("/objects/people", Default::default());
                    }
                    Err(message) => {
                        error.set(Some(message));
                        loading.set(false);
                    }
                }
            });
        }
    };
    let forgot_password = move || {
        let email_value = email.get();
        if !email_value.trim().is_empty() {
            error.set(None);
            leptos::task::spawn_local(async move {
                match send_password_reset(&email_value).await {
                    Ok(()) => notice.set(Some(
                        "If this email is registered, a password reset link has been sent"
                            .to_string(),
                    )),
                    Err(message) => error.set(Some(message)),
                }
            });
        }
    };

    view! {
        <main class="auth-layout">
            <section
                class=move || if step.get() == AuthStep::Password { "auth-card auth-card-password" } else { "auth-card" }
                aria-label="Welcome to Nudge"
            >
                <div class="auth-brand">
                    <a href="/welcome" class="auth-logo" inner_html=LOGO_MARK></a>
                    <span class="auth-workspace-badge-wrap"><span class="auth-workspace-badge"><span>"N"</span></span></span>
                </div>
                <div class="auth-heading"><h1>"Welcome, Nudge."</h1></div>
                <form class="auth-form" on:submit=move |event| {
                    event.prevent_default();
                    submit();
                }>
                    <div class="auth-field">
                        <input class="t-input auth-input" autocomplete="email"
                            placeholder="Email" bind:value=email />
                    </div>
                    {move || match step.get() {
                        AuthStep::Email => ().into_any(),
                        AuthStep::Password => view! {
                            <div class="auth-password">
                                <input
                                    class="t-input auth-input"
                                    type=move || if password_visible.get() { "text" } else { "password" }
                                    autocomplete="off"
                                    aria-describedby=move || (!user_exists.get()).then_some("auth-password-hint")
                                    placeholder="Password"
                                    bind:value=password
                                />
                                <div class="auth-password-reveal">
                                    <div
                                        data-testid="reveal-password-button"
                                        class="auth-password-toggle"
                                        on:click=move |_| password_visible.update(|visible| *visible = !*visible)
                                    >
                                        {move || {
                                            if password_visible.get() {
                                                view! { <Icon name="eye-off" size=16/> }.into_any()
                                            } else {
                                                view! { <Icon name="eye" size=16/> }.into_any()
                                            }
                                        }}
                                    </div>
                                </div>
                            </div>
                            {if user_exists.get() {
                                None
                            } else {
                                Some(view! { <p id="auth-password-hint" class="auth-password-hint">{"At least 8 characters long."}</p> })
                            }}
                        }.into_any(),
                    }}
                    {move || error.get().map(|message| view! { <div class="auth-error" role="alert">{message}</div> })}
                    <div class="auth-submit-row">
                        <button
                            class="t-button auth-submit"
                            type="submit"
                            disabled=move || {
                                loading.get()
                                    || (step.get() == AuthStep::Password
                                        && password.get().chars().count() < 8)
                            }
                            data-variant="solid"
                            data-color="neutral"
                            data-size="md"
                            data-full-width="true"
                            data-elevated="true"
                        >
                            <span class="t-button-content">
                                <span class="t-button-label">
                                    {move || {
                                        if loading.get() {
                                            "Please wait...".to_string()
                                        } else {
                                            match step.get() {
                                                AuthStep::Email => "Continue".to_string(),
                                                AuthStep::Password if user_exists.get() => "Sign in".to_string(),
                                                AuthStep::Password => "Sign up".to_string(),
                                            }
                                        }
                                    }}
                                </span>
                            </span>
                        </button>
                    </div>
                </form>
                <Show when=move || step.get() == AuthStep::Password fallback=|| ()>
                    <button
                        type="button"
                        class="auth-forgot"
                        data-variant="link"
                        data-color="neutral"
                        data-size="md"
                        on:click=move |_| forgot_password()
                    >
                        <span class="t-button-content">
                            <span class="t-button-label">{"Forgot your password?"}</span>
                        </span>
                    </button>
                </Show>
                <Show when=move || step.get() == AuthStep::Email fallback=|| ()>
                    <div class="auth-footer">
                        <a href="/privacy-policy" target="_blank" rel="noopener noreferrer">"Privacy Policy"</a>
                        <span>"•"</span>
                        <a href="/terms" target="_blank" rel="noopener noreferrer">"Terms of Service"</a>
                    </div>
                </Show>
            </section>
            <Show when=move || notice.get().is_some() fallback=|| ()>
                <ToastViewport>
                    <Toast
                        title=notice.get().unwrap_or_default()
                        tone=ToastTone::Success
                        duration=6000
                        on_dismiss=move |_| notice.set(None)
                    />
                </ToastViewport>
            </Show>
        </main>
    }
}

struct Session {
    token: String,
    workspace_id: String,
}

async fn check_user(email: &str) -> Result<bool, String> {
    let response = BrowserGraphQLClient::default()
        .post_json(&server_url(), "/auth/check-user", json!({ "email": email }))
        .await
        .map_err(|error| error.to_string())?;
    response["exists"]
        .as_bool()
        .ok_or_else(|| "Unable to check account status".to_string())
}

async fn sign_up(email: &str, password: &str) -> Result<Session, String> {
    let response = BrowserGraphQLClient::default()
        .post_json(
            &server_url(),
            "/auth/sign-up",
            json!({
                "email": email,
                "password": password,
                "workspaceDisplayName": "Nudge",
                "locale": "en"
            }),
        )
        .await
        .map_err(|error| error.to_string())?;
    let token = response["accessToken"]
        .as_str()
        .ok_or_else(|| "Sign-up response did not include an access token".to_string())?
        .to_string();
    let workspace_id = response["workspaceId"]
        .as_str()
        .ok_or_else(|| "Sign-up response did not include a workspace".to_string())?
        .to_string();
    Ok(Session {
        token,
        workspace_id,
    })
}

async fn send_password_reset(email: &str) -> Result<(), String> {
    BrowserGraphQLClient::default()
        .post_json(
            &server_url(),
            "/auth/email-password-reset-link",
            json!({ "email": email }),
        )
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

async fn sign_in(email: &str, password: &str) -> Result<Session, String> {
    let base_url = server_url();
    let client = BrowserGraphQLClient::default();
    let email_json = serde_json::to_string(email).map_err(|error| error.to_string())?;
    let password_json = serde_json::to_string(password).map_err(|error| error.to_string())?;
    let query = format!(
        "mutation signIn {{ signIn(email: {email_json}, password: {password_json}) {{ availableWorkspaces {{ availableWorkspacesForSignIn {{ id loginToken }} }} }} }}"
    );
    let response = client
        .execute_data(
            &base_url,
            &query,
            RequestOptions::new(Endpoint::Metadata).operation_name("signIn"),
        )
        .await
        .map_err(|error| error.to_string())?;
    let workspace = &response["signIn"]["availableWorkspaces"]["availableWorkspacesForSignIn"][0];
    let workspace_id = workspace["id"].as_str().unwrap_or_default().to_string();
    let login_token = workspace["loginToken"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let origin = web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .unwrap_or_else(|| base_url.clone());
    let exchange = format!(
        "mutation ex {{ getAuthTokensFromLoginToken(loginToken: {}, origin: {}) {{ tokens {{ accessOrWorkspaceAgnosticToken {{ token }} }} }} }}",
        serde_json::to_string(&login_token).map_err(|error| error.to_string())?,
        serde_json::to_string(&origin).map_err(|error| error.to_string())?
    );
    let exchanged = client
        .execute_data(
            &base_url,
            &exchange,
            RequestOptions::new(Endpoint::Metadata).operation_name("ex"),
        )
        .await
        .map_err(|error| error.to_string())?;
    let token = exchanged["getAuthTokensFromLoginToken"]["tokens"]
        ["accessOrWorkspaceAgnosticToken"]["token"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if token.is_empty() || workspace_id.is_empty() {
        return Err(String::from("Unable to sign in to this workspace"));
    }
    Ok(Session {
        token,
        workspace_id,
    })
}

async fn fetch_current_user(token: &str) -> Result<serde_json::Value, String> {
    let client = BrowserGraphQLClient::default();
    client
        .execute_data(
            &server_url(),
            "query { currentUser { id email } }",
            RequestOptions::new(Endpoint::Metadata).bearer_token(token),
        )
        .await
        .map_err(|error| error.to_string())
}
