use leptos::callback::Callable;
use leptos::prelude::*;

use crate::components::button::join_classes;

pub const TEXTAREA_CLASS: &str = "t-textarea";

#[component]
pub fn Textarea(
    #[prop(optional, into)] value: Signal<String>,
    #[prop(optional, into)] on_input: Option<Callback<String>>,
    #[prop(optional, into)] on_change: Option<Callback<String>>,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] rows: Option<u32>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] required: bool,
    #[prop(optional)] name: Option<&'static str>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let root_class = join_classes(&[
        Some(TEXTAREA_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    view! {
        <textarea
            class=root_class
            name=name
            placeholder=placeholder
            rows=rows
            prop:value=value
            required=required
            disabled=disabled
            on:input=move |event| {
                if !disabled {
                    if let Some(on_input) = on_input {
                        on_input.run(event_target_value(&event));
                    }
                }
            }
            on:change=move |event| {
                if !disabled {
                    if let Some(on_change) = on_change {
                        on_change.run(event_target_value(&event));
                    }
                }
            }
        >{move || value.get()}</textarea>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textarea_renders_native_attributes_and_custom_class() {
        let out = (view! {
            <Textarea
                value="Notes"
                placeholder="Add notes"
                rows=5
                name="notes"
                required=true
                disabled=true
                class="notes-field"
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-textarea notes-field\""), "{out}");
        assert!(out.contains("name=\"notes\""), "{out}");
        assert!(out.contains("placeholder=\"Add notes\""), "{out}");
        assert!(out.contains("rows=\"5\""), "{out}");
        assert!(out.contains(">Notes</textarea>"), "{out}");
        assert!(out.contains(" required"), "{out}");
        assert!(out.contains(" disabled"), "{out}");
    }

    #[test]
    fn controlled_value_and_callbacks_compile_through_props_builder() {
        let (value, set_value) = signal(String::from("Ada"));
        let props = TextareaProps::builder()
            .value(value)
            .on_input(move |next: String| set_value.set(next))
            .on_change(move |next: String| set_value.set(next))
            .build();
        let out = Textarea(props).into_view().to_html();
        assert!(out.contains(">Ada</textarea>"), "{out}");

        set_value.set(String::from("Grace"));
        let out = Textarea(TextareaProps::builder().value(value).build())
            .into_view()
            .to_html();
        assert!(out.contains(">Grace</textarea>"), "{out}");
        assert!(!out.contains(">Ada</textarea>"), "{out}");
    }

    #[test]
    fn default_textarea_is_empty_without_optional_attributes() {
        let out = (view! { <Textarea/> }).into_view().to_html();
        assert!(out.contains("class=\"t-textarea\""), "{out}");
        assert!(out.contains("></textarea>"), "{out}");
        assert!(!out.contains("rows="), "{out}");
        assert!(!out.contains("name="), "{out}");
        assert!(!out.contains("required"), "{out}");
        assert!(!out.contains("disabled"), "{out}");
    }
}
