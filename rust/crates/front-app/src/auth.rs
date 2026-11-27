use front_core::client::BrowserGraphQLClient;
use front_core::client::Endpoint;
use front_core::client::GraphQLClient;
use front_core::client::RequestOptions;
use front_ui::icons::IconSun;
use front_ui::tokens::Scheme;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

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
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);

    let submit = move || match step.get() {
        AuthStep::Email => {
            if !email.get().trim().is_empty() {
                error.set(None);
                step.set(AuthStep::Password);
            }
        }
        AuthStep::Password => {
            let email = email.get();
            let password = password.get();
            let auth = auth.clone();
            let navigate = navigate.clone();
            error.set(None);
            loading.set(true);
            leptos::task::spawn_local(async move {
                match sign_in(&email, &password).await {
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

    view! {
        <main class="auth-layout">
            <section class="auth-card" aria-label="Welcome to Nudge">
                <div class="auth-brand">
                    <div class="auth-logo" inner_html=LOGO_MARK></div>
                    <span class="auth-workspace-badge">"N"</span>
                </div>
                <div class="auth-heading"><h1>"Welcome, Nudge."</h1></div>
                <form class="auth-form" on:submit=move |event| {
                    event.prevent_default();
                    submit();
                }>
                    {move || match step.get() {
                        AuthStep::Email => view! {
                            <input class="t-input auth-input" type="email" autocomplete="email" required
                                placeholder="Email" bind:value=email />
                        }.into_any(),
                        AuthStep::Password => view! {
                            <input class="t-input auth-input" type="password" autocomplete="current-password" required
                                placeholder="Password" bind:value=password />
                        }.into_any(),
                    }}
                    {move || error.get().map(|message| view! { <div class="auth-error" role="alert">{message}</div> })}
                    <button class="t-button auth-submit" type="submit" disabled=move || loading.get()
                        data-variant="solid" data-color="neutral" data-size="md" data-full-width="true" data-elevated="true">
                        {move || if loading.get() { "Signing in..." } else { "Continue" }}
                    </button>
                </form>
                <Show when=move || step.get() == AuthStep::Password fallback=|| view! { <span /> }>
                    <button class="auth-back" on:click=move |_| {
                        error.set(None);
                        step.set(AuthStep::Email);
                    }>"Back"</button>
                </Show>
                <footer class="auth-footer">
                    <a href="/privacy-policy">"Privacy Policy"</a>
                    <span>"•"</span>
                    <a href="/terms">"Terms of Service"</a>
                </footer>
            </section>
        </main>
    }
}

struct Session {
    token: String,
    workspace_id: String,
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
    let exchange = format!(
        "mutation ex {{ getAuthTokensFromLoginToken(loginToken: {}, origin: \"http://refhq.localhost:3001\") {{ tokens {{ accessOrWorkspaceAgnosticToken {{ token }} }} }} }}",
        serde_json::to_string(&login_token).map_err(|error| error.to_string())?
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
            RequestOptions::new(Endpoint::Metadata)
                .operation_name("currentUser")
                .bearer_token(token),
        )
        .await
        .map_err(|error| error.to_string())
}
