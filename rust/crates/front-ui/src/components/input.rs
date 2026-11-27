use leptos::callback::Callable;
use leptos::prelude::*;

use crate::components::button::join_classes;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InputSize {
    Sm,
    #[default]
    Md,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InputVariant {
    #[default]
    Default,
    Plain,
}

impl InputSize {
    pub fn as_str(self) -> &'static str {
        match self {
            InputSize::Sm => "sm",
            InputSize::Md => "md",
        }
    }

    pub fn class(self) -> &'static str {
        match self {
            InputSize::Sm => "t-input-sm",
            InputSize::Md => "t-input-md",
        }
    }
}

impl InputVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            InputVariant::Default => "default",
            InputVariant::Plain => "plain",
        }
    }
}

pub const INPUT_CLASS: &str = "t-input";
pub const INPUT_GROUP_ROOT_CLASS: &str = "t-inputGroup-root";
pub const INPUT_GROUP_START_CLASS: &str = "t-inputGroup-startElement";
pub const INPUT_GROUP_END_CLASS: &str = "t-inputGroup-endElement";

#[component]
pub fn Input(
    #[prop(optional)] size: Option<InputSize>,
    #[prop(optional)] variant: Option<InputVariant>,
    #[prop(default = true)] has_focus_ring: bool,
    #[prop(optional)] grouped: bool,
    #[prop(optional)] input_type: &'static str,
    #[prop(optional)] name: Option<&'static str>,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional, into)] value: Option<Signal<String>>,
    #[prop(optional)] required: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional, into)] on_input: Option<Callback<String>>,
    #[prop(optional, into)] on_change: Option<Callback<String>>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let size = size.unwrap_or_default();
    let value = value.unwrap_or_default();
    let variant = if grouped {
        InputVariant::Plain
    } else {
        variant.unwrap_or_default()
    };
    let input_type = if input_type.is_empty() {
        "text"
    } else {
        input_type
    };
    let root_class = join_classes(&[
        Some(INPUT_CLASS),
        Some(size.class()),
        if class.is_empty() { None } else { Some(class) },
    ]);
    view! {
        <input
            class=root_class
            type=input_type
            name=name
            placeholder=placeholder
            value=value
            required=required
            disabled=disabled
            data-variant=variant.as_str()
            data-focus-ring=if has_focus_ring { "true" } else { "false" }
            data-grouped=grouped.then_some("")
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
        />
    }
}

#[component]
pub fn InputGroup(
    children: Children,
    #[prop(optional)] size: Option<InputSize>,
    #[prop(optional)] start_element: Option<AnyView>,
    #[prop(optional)] end_element: Option<AnyView>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let size = size.unwrap_or_default();
    let size_class = format!("t-inputGroup-{}", size.as_str());
    let root_class = join_classes(&[
        Some(INPUT_GROUP_ROOT_CLASS),
        Some(size_class.as_str()),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let start = start_element;
    let end = end_element;
    view! {
        <div class=root_class data-size=size.as_str()>
            {start.map(|node| view! { <span class=INPUT_GROUP_START_CLASS>{node}</span> })}
            {children()}
            {end.map(|node| view! { <span class=INPUT_GROUP_END_CLASS>{node}</span> })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_input_is_md_with_focus_ring() {
        let out = (view! { <Input/> }).into_view().to_html();
        assert!(out.contains("class=\"t-input t-input-md\""), "{out}");
        assert!(out.contains("type=\"text\""), "{out}");
        assert!(out.contains("data-variant=\"default\""), "{out}");
        assert!(out.contains("data-focus-ring=\"true\""), "{out}");
        assert!(out.contains("value=\"\""), "{out}");
        assert!(!out.contains("data-grouped"), "{out}");
        assert!(!out.contains("name="), "{out}");
        assert!(!out.contains("required"), "{out}");
        assert!(!out.contains("disabled"), "{out}");
    }

    #[test]
    fn native_input_attributes_render() {
        let out = (view! {
            <Input
                input_type="email"
                name="email"
                placeholder="Email address"
                value="ada@example.com"
                required=true
                disabled=true
            />
        })
        .into_view()
        .to_html();
        assert!(out.contains("type=\"email\""), "{out}");
        assert!(out.contains("name=\"email\""), "{out}");
        assert!(out.contains("placeholder=\"Email address\""), "{out}");
        assert!(out.contains("value=\"ada@example.com\""), "{out}");
        assert!(out.contains(" required"), "{out}");
        assert!(out.contains(" disabled"), "{out}");
    }

    #[test]
    fn controlled_value_and_callbacks_compile_through_props_builder() {
        let (value, set_value) = signal(String::from("Ada"));
        let props = InputProps::builder()
            .value(value)
            .on_input(move |next: String| set_value.set(next))
            .on_change(move |next: String| set_value.set(next))
            .build();
        let out = Input(props).into_view().to_html();
        assert!(out.contains("value=\"Ada\""), "{out}");

        set_value.set(String::from("Grace"));
        let out = Input(InputProps::builder().value(value).build())
            .into_view()
            .to_html();
        assert!(out.contains("value=\"Grace\""), "{out}");
        assert!(!out.contains("value=\"Ada\""), "{out}");
    }

    #[test]
    fn grouped_input_switches_to_plain_variant() {
        let out = (view! { <Input grouped=true size=InputSize::Sm/> })
            .into_view()
            .to_html();
        assert!(out.contains("data-variant=\"plain\""), "{out}");
        assert!(out.contains("t-input-sm"), "{out}");
        assert!(out.contains("data-grouped"), "{out}");
    }

    #[test]
    fn input_group_wraps_elements_in_classed_spans() {
        let out = (view! {
            <InputGroup
                size=InputSize::Sm
                start_element=view! { <b class="s"/> }.into_any()
                end_element=view! { <i class="e"/> }.into_any()
            >
                <Input grouped=true/>
            </InputGroup>
        })
        .into_view()
        .to_html();
        assert!(out.contains(INPUT_GROUP_ROOT_CLASS), "{out}");
        assert!(out.contains("t-inputGroup-sm"), "{out}");
        assert!(out.contains(INPUT_GROUP_START_CLASS), "{out}");
        assert!(out.contains(INPUT_GROUP_END_CLASS), "{out}");
        assert!(out.contains("data-size=\"sm\""), "{out}");
    }
}
