pub const DEFAULT_WORKSPACE_LOGO: &str = "/icons/nudge-mark-48.png";

const ABSOLUTE_SCHEMES: [&str; 4] = ["http:", "https:", "data:", "blob:"];

fn url_origin(base_url: &str) -> Option<String> {
    let scheme_end = base_url.find("://")?;
    let scheme = &base_url[..scheme_end];
    if scheme.is_empty() {
        return None;
    }
    let rest = &base_url[scheme_end + 3..];
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    if rest[..authority_end].is_empty() {
        return None;
    }
    Some(format!("{}://{}", scheme, &rest[..authority_end]))
}

fn lower_has_scheme(image_url: &str) -> bool {
    let lowered = image_url.to_lowercase();
    ABSOLUTE_SCHEMES
        .iter()
        .any(|scheme| lowered.starts_with(scheme))
        || image_url.starts_with("//")
}

pub fn image_absolute_uri(image_url: &str, base_url: &str) -> Option<String> {
    if lower_has_scheme(image_url) {
        return Some(image_url.to_string());
    }
    let origin = url_origin(base_url)?;
    if image_url.starts_with('/') {
        Some(format!("{origin}/files{image_url}"))
    } else {
        Some(format!("{origin}/files/{image_url}"))
    }
}

pub fn favicon_href(workspace_logo: Option<&str>, base_url: &str) -> String {
    workspace_logo
        .filter(|logo| !logo.is_empty())
        .and_then(|logo| image_absolute_uri(logo, base_url))
        .unwrap_or_else(|| DEFAULT_WORKSPACE_LOGO.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_logo_urls_are_passed_through() {
        for logo in [
            "https://cdn.example.com/a.png",
            "HTTPS://CDN.EXAMPLE.COM/a.png",
            "http://cdn.example.com/a.png",
            "data:image/png;base64,AAA",
            "blob:https://a/1",
            "//cdn.example.com/a.png",
        ] {
            assert_eq!(
                image_absolute_uri(logo, "http://127.0.0.1:3000").as_deref(),
                Some(logo)
            );
        }
    }

    #[test]
    fn relative_logos_are_served_from_the_files_route() {
        assert_eq!(
            image_absolute_uri("/uploads/logo.png", "http://127.0.0.1:3000").as_deref(),
            Some("http://127.0.0.1:3000/files/uploads/logo.png")
        );
        assert_eq!(
            image_absolute_uri("uploads/logo.png", "http://127.0.0.1:3000/").as_deref(),
            Some("http://127.0.0.1:3000/files/uploads/logo.png")
        );
    }

    #[test]
    fn the_base_origin_drops_any_inherited_path() {
        assert_eq!(
            image_absolute_uri("logo.png", "http://127.0.0.1:3000/front/prefix").as_deref(),
            Some("http://127.0.0.1:3000/files/logo.png")
        );
    }

    #[test]
    fn an_unusable_base_url_yields_no_absolute_uri() {
        assert_eq!(image_absolute_uri("logo.png", "not-a-url"), None);
        assert_eq!(image_absolute_uri("logo.png", "://x"), None);
    }

    #[test]
    fn the_favicon_falls_back_to_the_default_logo() {
        assert_eq!(
            favicon_href(None, "http://127.0.0.1:3000"),
            DEFAULT_WORKSPACE_LOGO
        );
        assert_eq!(
            favicon_href(Some(""), "http://127.0.0.1:3000"),
            DEFAULT_WORKSPACE_LOGO
        );
        assert_eq!(
            favicon_href(Some("logo.png"), "broken"),
            DEFAULT_WORKSPACE_LOGO
        );
        assert_eq!(
            favicon_href(Some("logo.png"), "http://127.0.0.1:3000"),
            "http://127.0.0.1:3000/files/logo.png"
        );
    }
}
