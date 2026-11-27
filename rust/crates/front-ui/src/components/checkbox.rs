use leptos::callback::Callable;
use leptos::prelude::*;

use crate::components::button::join_classes;

pub const CHECKBOX_CLASS: &str = "t-checkbox";
pub const CHECKBOX_INPUT_CLASS: &str = "t-checkbox-input";
pub const CHECKBOX_LABEL_CLASS: &str = "t-checkbox-label";

#[component]
pub fn Checkbox(
    #[prop(optional, into)] checked: Signal<bool>,
    #[prop(optional, into)] on_change: Option<Callback<bool>>,
    #[prop(optional)] label: &'static str,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] required: bool,
    #[prop(optional)] name: Option<&'static str>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let root_class = join_classes(&[
        Some(CHECKBOX_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    view! {
        <label class=root_class>
            <input
                class=CHECKBOX_INPUT_CLASS
                type="checkbox"
                name=name
                checked=checked
                prop:checked=checked
                required=required
                disabled=disabled
                on:change=move |event| {
                    if !disabled {
                        if let Some(on_change) = on_change {
                            on_change.run(event_target_checked(&event));
                        }
                    }
                }
            />
            <span class=CHECKBOX_LABEL_CLASS>{label}</span>
        </label>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkbox_renders_native_attributes_and_associated_label() {
        let out = (view! {
            <Checkbox
                checked=true
                label="Accept terms"
                name="terms"
                required=true
                disabled=true
                class="terms-check"
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-checkbox terms-check\""), "{out}");
        assert!(out.contains("type=\"checkbox\""), "{out}");
        assert!(out.contains("name=\"terms\""), "{out}");
        assert!(out.contains(" checked"), "{out}");
        assert!(out.contains(" required"), "{out}");
        assert!(out.contains(" disabled"), "{out}");
        assert!(out.contains(CHECKBOX_LABEL_CLASS), "{out}");
        assert!(out.contains(">Accept terms<"), "{out}");
    }

    #[test]
    fn checked_signal_and_change_callback_compile_through_props_builder() {
        let (checked, set_checked) = signal(false);
        let props = CheckboxProps::builder()
            .checked(checked)
            .on_change(move |next: bool| set_checked.set(next))
            .label("Subscribe")
            .build();
        let out = Checkbox(props).into_view().to_html();
        assert!(!out.contains(" checked"), "{out}");

        set_checked.set(true);
        let out = Checkbox(CheckboxProps::builder().checked(checked).build())
            .into_view()
            .to_html();
        assert!(out.contains(" checked"), "{out}");
    }

    #[test]
    fn default_checkbox_is_unchecked_and_enabled() {
        let out = (view! { <Checkbox/> }).into_view().to_html();
        assert!(out.contains("class=\"t-checkbox\""), "{out}");
        assert!(!out.contains(" checked"), "{out}");
        assert!(!out.contains("required"), "{out}");
        assert!(!out.contains("disabled"), "{out}");
        assert!(!out.contains("name="), "{out}");
    }
}
