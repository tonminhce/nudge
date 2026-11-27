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

const LOGO_LIGHT: &str = include_str!("../assets/nudge-logo-light.svg");
const LOGO_DARK: &str = include_str!("../assets/nudge-logo-dark.svg");

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

#[component]
pub fn AuthPage() -> impl IntoView {
    let auth = expect_context::<AuthState>();
    let theme = expect_context::<RwSignal<Scheme>>();
    let navigate = use_navigate();
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let loading = RwSignal::new(false);
    let mode = RwSignal::new(AuthMode::SignIn);

    let submit = move || {
        let email = email.get();
        let password = password.get();
        let mode = mode.get();
        let auth = auth.clone();
        let navigate = navigate.clone();
        error.set(None);
        loading.set(true);
        leptos::task::spawn_local(async move {
            let result = match mode {
                AuthMode::SignIn => sign_in(&email, &password).await,
                AuthMode::SignUp => sign_up(&email, &password).await,
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
    };

    view! {
        <main class="auth-layout">
            <div class="auth-theme"><ThemeToggle /></div>
            <section class="auth-card" aria-label="Welcome to Nudge">
                <div class="auth-brand">
                    <div class="auth-logo" inner_html=move || match theme.get() {
                        Scheme::Light => LOGO_LIGHT,
                        Scheme::Dark => LOGO_DARK,
                    }></div>
                </div>
                <div class="auth-heading">
                    <h1>{move || match mode.get() { AuthMode::SignIn => "Welcome to Nudge", AuthMode::SignUp => "Create your workspace" }}</h1>
                    <p>{move || match mode.get() { AuthMode::SignIn => "Sign in to your workspace", AuthMode::SignUp => "Start your 14-day free trial" }}</p>
                </div>
                <form class="auth-form" on:submit=move |event| {
                    event.prevent_default();
                    submit();
                }>
                    <label class="auth-field">
                        <span>Email</span>
                        <input class="t-input t-input-md" type="email" autocomplete="email" required
                            placeholder="name@company.com" bind:value=email />
                    </label>
                    <label class="auth-field">
                        <span>Password</span>
                        <input class="t-input t-input-md" type="password" autocomplete="current-password" required
                            placeholder="Enter your password" bind:value=password />
                    </label>
                    {move || error.get().map(|message| view! { <div class="auth-error" role="alert">{message}</div> })}
                    <button class="t-button auth-submit" type="submit" disabled=move || loading.get()
                        data-variant="solid" data-color="accent" data-size="md">
                        {move || if loading.get() { "Signing in..." } else { "Continue" }}
                    </button>
                </form>
                <div class="auth-divider"><span>or continue with</span></div>
                <div class="auth-sso">
                    <button class="t-button auth-sso-button" data-variant="outline" data-color="neutral" data-size="md">
                        <span class="google-mark">G</span> Google
                    </button>
                    <button class="t-button auth-sso-button" data-variant="outline" data-color="neutral" data-size="md">
                        <span class="microsoft-mark"><i></i><i></i><i></i><i></i></span> Microsoft
                    </button>
                </div>
                <p class="auth-switch">
                    {move || match mode.get() { AuthMode::SignIn => "Don't have an account?", AuthMode::SignUp => "Already have an account?" }}
                    <button on:click=move |_| mode.set(match mode.get() { AuthMode::SignIn => AuthMode::SignUp, AuthMode::SignUp => AuthMode::SignIn })>
                        {move || match mode.get() { AuthMode::SignIn => "Sign up", AuthMode::SignUp => "Sign in" }}
                    </button>
                </p>
            </section>
            <footer class="auth-footer">Nudge CRM - Open source customer relationship management</footer>
        </main>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AuthMode {
    SignIn,
    SignUp,
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

async fn sign_up(email: &str, password: &str) -> Result<Session, String> {
    let base_url = server_url();
    let client = BrowserGraphQLClient::default();
    let email_json = serde_json::to_string(email).map_err(|error| error.to_string())?;
    let password_json = serde_json::to_string(password).map_err(|error| error.to_string())?;
    let query = format!(
        "mutation signUp {{ signUp(email: {email_json}, password: {password_json}) {{ tokens {{ accessOrWorkspaceAgnosticToken {{ token }} }} }} }}"
    );
    let response = client
        .execute_data(
            &base_url,
            &query,
            RequestOptions::new(Endpoint::Metadata).operation_name("signUp"),
        )
        .await
        .map_err(|error| error.to_string())?;
    let token = response["signUp"]["tokens"]["accessOrWorkspaceAgnosticToken"]["token"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let workspace_id = response["signUp"]["workspace"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    if token.is_empty() {
        return Err(String::from("Unable to create the workspace"));
    }
    Ok(Session {
        token,
        workspace_id,
    })
}
