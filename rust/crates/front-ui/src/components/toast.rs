use leptos::callback::Callable;
use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToastTone {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}

impl ToastTone {
    pub fn as_str(self) -> &'static str {
        match self {
            ToastTone::Info => "info",
            ToastTone::Success => "success",
            ToastTone::Warning => "warning",
            ToastTone::Error => "error",
        }
    }

    pub fn is_alert(self) -> bool {
        self == ToastTone::Error
    }
}

pub const TOAST_CLASS: &str = "t-toast";
pub const TOAST_CONTENT_CLASS: &str = "t-toast-content";
pub const TOAST_TITLE_CLASS: &str = "t-toast-title";
pub const TOAST_DESCRIPTION_CLASS: &str = "t-toast-description";
pub const TOAST_BODY_CLASS: &str = "t-toast-body";
pub const TOAST_DURATION_CLASS: &str = "t-toast-duration";
pub const TOAST_DISMISS_CLASS: &str = "t-toast-dismiss";

#[component]
pub fn Toast(
    title: &'static str,
    #[prop(optional)] description: &'static str,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] tone: Option<ToastTone>,
    #[prop(optional, into)] on_dismiss: Option<Callback<()>>,
    #[prop(optional)] duration: Option<u32>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let tone = tone.unwrap_or_default();
    let root_class = join_classes(&[
        Some(TOAST_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let children_slot = children.map(|children| children().into_any());
    view! {
        <div
            class=root_class
            role=if tone.is_alert() { "alert" } else { "status" }
            aria-atomic="true"
            data-tone=tone.as_str()
            data-duration=duration.map(|duration| duration.to_string())
        >
            <div class=TOAST_CONTENT_CLASS>
                <div class=TOAST_TITLE_CLASS>{title}</div>
                {if description.is_empty() {
                    None
                } else {
                    Some(view! { <div class=TOAST_DESCRIPTION_CLASS>{description}</div> })
                }}
                {children_slot.map(|node| view! { <div class=TOAST_BODY_CLASS>{node}</div> })}
            </div>
            {duration.map(|duration| view! {
                <span class=TOAST_DURATION_CLASS>{format!("{duration} ms")}</span>
            })}
            {on_dismiss.map(|_| view! {
                <button
                    type="button"
                    class=TOAST_DISMISS_CLASS
                    aria-label="Dismiss"
                    on:click=move |_| {
                        if let Some(on_dismiss) = on_dismiss {
                            on_dismiss.run(());
                        }
                    }
                >
                    <span aria-hidden="true">{"×"}</span>
                </button>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_toast_is_a_polite_status_with_info_tone() {
        let out = (view! {
            <Toast title="Saved" description="Your changes are up to date"/>
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-toast\""), "{out}");
        assert!(out.contains("role=\"status\""), "{out}");
        assert!(out.contains("aria-atomic=\"true\""), "{out}");
        assert!(out.contains("data-tone=\"info\""), "{out}");
        assert!(out.contains(TOAST_TITLE_CLASS), "{out}");
        assert!(out.contains(">Saved<"), "{out}");
        assert!(out.contains(TOAST_DESCRIPTION_CLASS), "{out}");
        assert!(out.contains(">Your changes are up to date<"), "{out}");
        assert!(!out.contains("data-duration"), "{out}");
        assert!(!out.contains(TOAST_DISMISS_CLASS), "{out}");
    }

    #[test]
    fn error_toast_is_an_alert_and_renders_children() {
        let out = (view! {
            <Toast title="Upload failed" tone=ToastTone::Error class="upload-toast">
                {"Try again."}
            </Toast>
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-toast upload-toast\""), "{out}");
        assert!(out.contains("role=\"alert\""), "{out}");
        assert!(out.contains("data-tone=\"error\""), "{out}");
        assert!(out.contains(TOAST_BODY_CLASS), "{out}");
        assert!(out.contains(">Try again.<"), "{out}");
    }

    #[test]
    fn duration_and_dismiss_controls_are_displayed() {
        let out = (view! {
            <Toast
                title="Copied"
                tone=ToastTone::Success
                duration=5000
                on_dismiss=|| {}
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("role=\"status\""), "{out}");
        assert!(out.contains("data-tone=\"success\""), "{out}");
        assert!(out.contains("data-duration=\"5000\""), "{out}");
        assert!(out.contains(TOAST_DURATION_CLASS), "{out}");
        assert!(out.contains(">5000 ms<"), "{out}");
        assert!(out.contains("type=\"button\""), "{out}");
        assert!(out.contains("aria-label=\"Dismiss\""), "{out}");
        assert!(out.contains(TOAST_DISMISS_CLASS), "{out}");
    }

    #[test]
    fn tone_and_dismiss_callback_compile_through_props_builder() {
        let props = ToastProps::builder()
            .title("Warning")
            .tone(ToastTone::Warning)
            .on_dismiss(|| {})
            .build();
        let out = Toast(props).into_view().to_html();
        assert!(out.contains("data-tone=\"warning\""), "{out}");
        assert!(out.contains("role=\"status\""), "{out}");
        assert!(out.contains(TOAST_DISMISS_CLASS), "{out}");
    }
}
