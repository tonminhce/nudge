use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ModalSize {
    Sm,
    Compact,
    #[default]
    Md,
    Lg,
    Xl,
    Fullscreen,
}

impl ModalSize {
    pub fn as_str(self) -> &'static str {
        match self {
            ModalSize::Sm => "sm",
            ModalSize::Compact => "compact",
            ModalSize::Md => "md",
            ModalSize::Lg => "lg",
            ModalSize::Xl => "xl",
            ModalSize::Fullscreen => "fullscreen",
        }
    }

    pub fn width_token(self) -> &'static str {
        match self {
            ModalSize::Sm => "--t-modal-size-sm-width",
            ModalSize::Compact => "--t-modal-size-compact-width",
            ModalSize::Md => "--t-modal-size-md-width",
            ModalSize::Lg => "--t-modal-size-lg-width",
            ModalSize::Xl => "--t-modal-size-xl-width",
            ModalSize::Fullscreen => "--t-modal-size-fullscreen-width",
        }
    }
}

pub const DIALOG_BACKDROP_CLASS: &str = "t-dialog-backdrop";
pub const DIALOG_VIEWPORT_CLASS: &str = "t-dialog-viewport";
pub const DIALOG_POPUP_CLASS: &str = "t-dialog-popup";
pub const DIALOG_HEADER_CLASS: &str = "t-dialog-header";
pub const DIALOG_BODY_CLASS: &str = "t-dialog-body";
pub const DIALOG_FOOTER_CLASS: &str = "t-dialog-footer";
pub const DIALOG_TITLE_CLASS: &str = "t-dialog-title";
pub const DIALOG_DESCRIPTION_CLASS: &str = "t-dialog-description";

#[component]
pub fn Modal(
    title: &'static str,
    #[prop(optional)] description: &'static str,
    #[prop(optional)] size: Option<ModalSize>,
    #[prop(optional)] open: bool,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] footer: Option<AnyView>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let size = size.unwrap_or_default();
    let popup_class = join_classes(&[
        Some(DIALOG_POPUP_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let style = format!("inline-size: var({});", size.width_token());
    let children_slot = children.map(|c| c().into_any());
    let footer_slot = footer;
    view! {
        <div
            class=DIALOG_VIEWPORT_CLASS
            role="dialog"
            aria-modal="true"
            aria-label=title
            data-size=size.as_str()
            data-open=open.then_some("")
        >
            <div class=DIALOG_BACKDROP_CLASS></div>
            <div class=popup_class style=style data-size=size.as_str()>
                <div class=DIALOG_HEADER_CLASS>
                    <h2 class=DIALOG_TITLE_CLASS>{title}</h2>
                    {if description.is_empty() {
                        None
                    } else {
                        Some(view! { <p class=DIALOG_DESCRIPTION_CLASS>{description}</p> })
                    }}
                </div>
                <div class=DIALOG_BODY_CLASS>{children_slot}</div>
                {footer_slot.map(|node| view! { <div class=DIALOG_FOOTER_CLASS>{node}</div> })}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modal_uses_dialog_classes_and_modal_size_tokens() {
        let out = (view! {
            <Modal title="Confirm" size=ModalSize::Lg open=true description="Body copy">
                {"content"}
            </Modal>
        })
        .into_view()
        .to_html();
        assert!(out.contains("role=\"dialog\""), "{out}");
        assert!(out.contains("aria-modal=\"true\""), "{out}");
        assert!(out.contains(DIALOG_VIEWPORT_CLASS), "{out}");
        assert!(out.contains(DIALOG_POPUP_CLASS), "{out}");
        assert!(out.contains(DIALOG_BACKDROP_CLASS), "{out}");
        assert!(out.contains(DIALOG_HEADER_CLASS), "{out}");
        assert!(out.contains(DIALOG_TITLE_CLASS), "{out}");
        assert!(out.contains(">Confirm<"), "{out}");
        assert!(out.contains(DIALOG_DESCRIPTION_CLASS), "{out}");
        assert!(out.contains("data-size=\"lg\""), "{out}");
        assert!(
            out.contains("inline-size: var(--t-modal-size-lg-width)"),
            "{out}"
        );
        assert!(out.contains("data-open"), "{out}");
        assert!(out.contains(DIALOG_BODY_CLASS), "{out}");
        assert!(out.contains(">content<"), "{out}");
    }

    #[test]
    fn modal_default_size_is_md() {
        let out = (view! { <Modal title="T"/> }).into_view().to_html();
        assert!(out.contains("data-size=\"md\""), "{out}");
        assert!(out.contains("var(--t-modal-size-md-width)"), "{out}");
    }

    #[test]
    fn modal_footer_slot_renders() {
        let out = (view! {
            <Modal title="T" footer=view! { <div class="f">{"Foot"}</div> }.into_any() />
        })
        .into_view()
        .to_html();
        assert!(out.contains(DIALOG_FOOTER_CLASS), "{out}");
        assert!(out.contains(">Foot<"), "{out}");
    }

    #[test]
    fn every_modal_size_maps_to_a_token() {
        let expected = [
            (ModalSize::Sm, "--t-modal-size-sm-width"),
            (ModalSize::Compact, "--t-modal-size-compact-width"),
            (ModalSize::Md, "--t-modal-size-md-width"),
            (ModalSize::Lg, "--t-modal-size-lg-width"),
            (ModalSize::Xl, "--t-modal-size-xl-width"),
            (ModalSize::Fullscreen, "--t-modal-size-fullscreen-width"),
        ];
        for (size, token) in expected {
            assert_eq!(size.width_token(), token);
            assert!(
                crate::tokens::token(crate::tokens::Scheme::Light, token).is_some(),
                "{token} must exist in the token sheet"
            );
        }
    }
}
