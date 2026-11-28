use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

pub const APP_NAME: &str = "Nudge";

static SETTINGS_PREFIXES: [(&str, &str); 12] = [
    ("/settings/app-preferences", "Apps preferences - Settings"),
    ("/settings/billing", "Billing - Settings"),
    ("/settings/partner", "Partner - Settings"),
    ("/settings/experience", "Experience - Settings"),
    ("/settings/profile", "Profile - Settings"),
    ("/settings/objects", "Data model - Settings"),
    ("/settings/members", "Members - Settings"),
    ("/settings/developers", "Developers - Settings"),
    ("/settings/functions", "Functions - Settings"),
    ("/settings/integrations", "Integrations - Settings"),
    ("/settings/general", "General - Settings"),
    ("/settings/community", "Community - Settings"),
];

static EXACT_PATHS: [(&str, &str); 5] = [
    ("/verify", "Verify"),
    ("/welcome", "Sign in or Create an account"),
    ("/invite/:workspaceInviteHash", "Invite"),
    ("/workspace-activation", "Create Workspace"),
    ("/create/profile", "Create Profile"),
];

static TITLE_IDS: AtomicU64 = AtomicU64::new(0);

pub fn next_page_title_id() -> u64 {
    TITLE_IDS.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn pathname_prefix(pathname: &str) -> &str {
    SETTINGS_PREFIXES
        .iter()
        .find(|(prefix, _)| pathname.starts_with(prefix))
        .map(|(prefix, _)| *prefix)
        .unwrap_or(pathname)
}

pub fn page_title_from_path(pathname: &str) -> String {
    let pathname_or_prefix = pathname_prefix(pathname);
    EXACT_PATHS
        .iter()
        .find(|(path, _)| *path == pathname_or_prefix)
        .map(|(_, title)| (*title).to_string())
        .or_else(|| {
            SETTINGS_PREFIXES
                .iter()
                .find(|(prefix, _)| *prefix == pathname_or_prefix)
                .map(|(_, title)| (*title).to_string())
        })
        .unwrap_or_else(|| APP_NAME.to_string())
}

pub fn resolve_document_title(route_title: &str, stack: &[(u64, String)]) -> String {
    stack
        .last()
        .map(|(_, title)| title.clone())
        .unwrap_or_else(|| route_title.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_title_matches_the_upstream_path_table() {
        assert_eq!(page_title_from_path("/verify"), "Verify");
        assert_eq!(
            page_title_from_path("/welcome"),
            "Sign in or Create an account"
        );
        assert_eq!(
            page_title_from_path("/workspace-activation"),
            "Create Workspace"
        );
        assert_eq!(page_title_from_path("/create/profile"), "Create Profile");
        assert_eq!(page_title_from_path("/objects/people"), APP_NAME);
        assert_eq!(page_title_from_path("/"), APP_NAME);
        assert_eq!(page_title_from_path("/random"), APP_NAME);
    }

    #[test]
    fn invite_title_only_matches_the_route_pattern_literal() {
        assert_eq!(
            page_title_from_path("/invite/:workspaceInviteHash"),
            "Invite"
        );
        assert_eq!(page_title_from_path("/invite/acme"), APP_NAME);
    }

    #[test]
    fn settings_prefixes_win_over_the_default_and_keep_their_labels() {
        assert_eq!(
            page_title_from_path("/settings/objects/opportunities"),
            "Data model - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/profile"),
            "Profile - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/experience"),
            "Experience - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/app-preferences/google"),
            "Apps preferences - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/billing/plans"),
            "Billing - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/partner"),
            "Partner - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/members"),
            "Members - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/general"),
            "General - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/developers"),
            "Developers - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/functions"),
            "Functions - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/integrations"),
            "Integrations - Settings"
        );
        assert_eq!(
            page_title_from_path("/settings/community"),
            "Community - Settings"
        );
    }

    #[test]
    fn unmapped_settings_paths_fall_back_to_the_app_name() {
        assert_eq!(page_title_from_path("/settings"), APP_NAME);
        assert_eq!(page_title_from_path("/settings/roles"), APP_NAME);
    }

    #[test]
    fn settings_prefix_lookup_follows_the_declaration_order() {
        assert_eq!(
            pathname_prefix("/settings/members/roles"),
            "/settings/members"
        );
        assert_eq!(pathname_prefix("/objects/people"), "/objects/people");
    }

    #[test]
    fn document_title_uses_the_last_registered_page_title() {
        let stack = vec![
            (1, "People".to_string()),
            (2, "Ada Lovelace - Person".to_string()),
        ];
        assert_eq!(
            resolve_document_title(APP_NAME, &stack),
            "Ada Lovelace - Person"
        );
        assert_eq!(resolve_document_title(APP_NAME, &[]), APP_NAME);
    }

    #[test]
    fn title_ids_are_unique_and_start_at_one() {
        assert_eq!(next_page_title_id(), 1);
        assert_eq!(next_page_title_id(), 2);
    }
}
