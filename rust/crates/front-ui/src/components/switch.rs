use leptos::callback::Callable;
use leptos::prelude::*;

use crate::components::button::join_classes;

pub const SWITCH_CLASS: &str = "t-switch";
pub const SWITCH_CONTROL_CLASS: &str = "t-switch-control";
pub const SWITCH_THUMB_CLASS: &str = "t-switch-thumb";
pub const SWITCH_LABEL_CLASS: &str = "t-switch-label";

#[component]
pub fn Switch(
    #[prop(optional, into)] checked: Signal<bool>,
    #[prop(optional, into)] on_change: Option<Callback<bool>>,
    #[prop(optional)] label: &'static str,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let root_class = join_classes(&[
        Some(SWITCH_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let aria_label = (!label.is_empty()).then_some(label);
    view! {
        <label class=root_class>
            <button
                class=SWITCH_CONTROL_CLASS
                type="button"
                role="switch"
                aria-label=aria_label
                aria-checked=move || checked.get().to_string()
                disabled=disabled
                on:click=move |_| {
                    if !disabled {
                        if let Some(on_change) = on_change {
                            on_change.run(!checked.get());
                        }
                    }
                }
            >
                <span class=SWITCH_THUMB_CLASS aria-hidden="true"></span>
            </button>
            <span class=SWITCH_LABEL_CLASS>{label}</span>
        </label>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switch_renders_accessible_attributes_and_label() {
        let out = (view! {
            <Switch checked=true label="Notifications" disabled=true class="notification-switch"/>
        })
        .into_view()
        .to_html();
        assert!(
            out.contains("class=\"t-switch notification-switch\""),
            "{out}"
        );
        assert!(out.contains("type=\"button\""), "{out}");
        assert!(out.contains("role=\"switch\""), "{out}");
        assert!(out.contains("aria-checked=\"true\""), "{out}");
        assert!(out.contains("aria-label=\"Notifications\""), "{out}");
        assert!(out.contains(" disabled"), "{out}");
        assert!(out.contains(SWITCH_THUMB_CLASS), "{out}");
        assert!(out.contains("aria-hidden=\"true\""), "{out}");
        assert!(out.contains(">Notifications<"), "{out}");
    }

    #[test]
    fn checked_signal_and_change_callback_compile_through_props_builder() {
        let (checked, set_checked) = signal(false);
        let props = SwitchProps::builder()
            .checked(checked)
            .on_change(move |next: bool| set_checked.set(next))
            .label("Enabled")
            .build();
        let out = Switch(props).into_view().to_html();
        assert!(out.contains("aria-checked=\"false\""), "{out}");

        set_checked.set(true);
        let out = Switch(SwitchProps::builder().checked(checked).build())
            .into_view()
            .to_html();
        assert!(out.contains("aria-checked=\"true\""), "{out}");
    }

    #[test]
    fn default_switch_is_unchecked_and_enabled() {
        let out = (view! { <Switch/> }).into_view().to_html();
        assert!(out.contains("class=\"t-switch\""), "{out}");
        assert!(out.contains("aria-checked=\"false\""), "{out}");
        assert!(!out.contains("aria-label="), "{out}");
        assert!(!out.contains("disabled"), "{out}");
    }
}
