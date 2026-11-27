use leptos::callback::Callable;
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonVariant {
    Solid,
    Outline,
    Ghost,
    Soft,
    Link,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonColor {
    Neutral,
    Accent,
    Danger,
    Success,
    Inherit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonSize {
    Sm,
    Md,
}

impl ButtonVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            ButtonVariant::Solid => "solid",
            ButtonVariant::Outline => "outline",
            ButtonVariant::Ghost => "ghost",
            ButtonVariant::Soft => "soft",
            ButtonVariant::Link => "link",
        }
    }
}

impl ButtonColor {
    pub fn as_str(self) -> &'static str {
        match self {
            ButtonColor::Neutral => "neutral",
            ButtonColor::Accent => "accent",
            ButtonColor::Danger => "danger",
            ButtonColor::Success => "success",
            ButtonColor::Inherit => "inherit",
        }
    }
}

impl ButtonSize {
    pub fn as_str(self) -> &'static str {
        match self {
            ButtonSize::Sm => "sm",
            ButtonSize::Md => "md",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ButtonState {
    pub variant: ButtonVariant,
    pub color: ButtonColor,
    pub size: ButtonSize,
    pub full_width: bool,
    pub loading: bool,
    pub elevated: bool,
    pub disabled: bool,
}

impl ButtonState {
    pub fn new(
        variant: ButtonVariant,
        color: ButtonColor,
        size: ButtonSize,
        full_width: bool,
        loading: bool,
        elevated: bool,
        disabled: bool,
    ) -> Self {
        Self {
            variant,
            color,
            size,
            full_width,
            loading,
            elevated,
            disabled,
        }
    }
}

impl Default for ButtonState {
    fn default() -> Self {
        Self::new(
            ButtonVariant::Outline,
            ButtonColor::Neutral,
            ButtonSize::Md,
            false,
            false,
            false,
            false,
        )
    }
}

pub const BUTTON_CLASS: &str = "t-button";
pub const BUTTON_CONTENT_CLASS: &str = "t-button-content";
pub const BUTTON_LABEL_CLASS: &str = "t-button-label";
pub const BUTTON_ICON_CLASS: &str = "t-button-icon";
pub const BUTTON_LOADER_CLASS: &str = "t-button-loader";
pub const BUTTON_HIDDEN_CLASS: &str = "t-button-hidden";

pub fn join_classes(parts: &[Option<&str>]) -> String {
    parts
        .iter()
        .filter_map(|p| p.map(str::to_string))
        .collect::<Vec<_>>()
        .join(" ")
}

impl ButtonState {
    pub fn class(self) -> String {
        join_classes(&[
            Some(BUTTON_CLASS),
            self.elevated.then_some("t-button-elevated"),
        ])
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ButtonError {
    InheritSolid,
}

impl std::fmt::Display for ButtonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ButtonError::InheritSolid => write!(
                f,
                "Button color=\"inherit\" does not support variant=\"solid\". \
                 Use outline, ghost, soft, or link."
            ),
        }
    }
}

impl std::error::Error for ButtonError {}

pub fn resolve_button(
    variant: ButtonVariant,
    color: ButtonColor,
) -> Result<ButtonState, ButtonError> {
    if variant == ButtonVariant::Solid && color == ButtonColor::Inherit {
        return Err(ButtonError::InheritSolid);
    }
    Ok(ButtonState::new(
        variant,
        color,
        ButtonSize::Md,
        false,
        false,
        false,
        false,
    ))
}

#[component]
pub fn Button(
    children: Children,
    #[prop(optional)] variant: Option<ButtonVariant>,
    #[prop(optional)] color: Option<ButtonColor>,
    #[prop(optional)] size: Option<ButtonSize>,
    #[prop(optional)] full_width: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional)] elevated: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional, into)] on_click: Option<Callback<leptos::web_sys::MouseEvent>>,
    #[prop(optional)] start_icon: Option<AnyView>,
    #[prop(optional)] end_icon: Option<AnyView>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let state = ButtonState::new(
        variant.unwrap_or(ButtonVariant::Outline),
        color.unwrap_or(ButtonColor::Neutral),
        size.unwrap_or(ButtonSize::Md),
        full_width,
        loading,
        elevated,
        disabled,
    );
    let root_class = join_classes(&[
        Some(state.class().as_str()),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let content_class = if loading {
        join_classes(&[Some(BUTTON_CONTENT_CLASS), Some(BUTTON_HIDDEN_CLASS)])
    } else {
        BUTTON_CONTENT_CLASS.to_string()
    };
    let start = start_icon;
    let end = end_icon;
    let is_center_loading = loading;
    let effective_disabled = disabled || loading;
    view! {
        <button
            type="button"
            class=root_class
            data-variant=state.variant.as_str()
            data-color=state.color.as_str()
            data-size=state.size.as_str()
            data-full-width=state.full_width.then_some("")
            data-loading=state.loading.then_some("")
            data-elevated=state.elevated.then_some("")
            disabled=effective_disabled
            aria-busy=loading.then_some("true")
            on:click=move |event| {
                if !effective_disabled {
                    if let Some(on_click) = on_click {
                        on_click.run(event);
                    }
                }
            }
        >
            <span class=content_class>
                {start.map(|node| view! { <span class=BUTTON_ICON_CLASS aria-hidden="true">{node}</span> })}
                <span class=BUTTON_LABEL_CLASS>{children()}</span>
                {end.map(|node| view! { <span class=BUTTON_ICON_CLASS aria-hidden="true">{node}</span> })}
            </span>
            {is_center_loading.then(|| {
                view! { <span class=BUTTON_LOADER_CLASS aria-hidden="true"></span> }
            })}
        </button>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(state: ButtonState, label: &str) -> String {
        let label = label.to_string();
        let app = view! {
            <Button
                variant=state.variant
                color=state.color
                size=state.size
                full_width=state.full_width
                loading=state.loading
                elevated=state.elevated
                disabled=state.disabled
            >
                {label}
            </Button>
        };
        app.into_view().to_html()
    }

    fn sample() -> ButtonState {
        ButtonState::default()
    }

    #[test]
    fn default_button_emits_outline_neutral_md() {
        let html = render(sample(), "Save");
        assert!(html.contains("data-variant=\"outline\""), "{html}");
        assert!(html.contains("data-color=\"neutral\""), "{html}");
        assert!(html.contains("data-size=\"md\""), "{html}");
        assert!(html.contains(BUTTON_CLASS), "{html}");
        assert!(html.contains(BUTTON_LABEL_CLASS), "{html}");
        assert!(html.contains(">Save<"), "{html}");
        assert!(!html.contains("data-full-width"), "{html}");
        assert!(!html.contains("data-loading"), "{html}");
        assert!(!html.contains("disabled"), "{html}");
    }

    #[test]
    fn click_callback_compiles_and_disabled_renders_natively() {
        let html = (view! {
            <Button disabled=true on_click=move |_: leptos::web_sys::MouseEvent| {}>
                "Save"
            </Button>
        })
        .into_view()
        .to_html();
        assert!(html.contains("type=\"button\""), "{html}");
        assert!(html.contains("disabled"), "{html}");
        assert!(html.contains(">Save<"), "{html}");
    }

    #[test]
    fn props_builder_accepts_click_callback() {
        let props = ButtonProps::builder()
            .children(ToChildren::to_children(|| "Save"))
            .on_click(|_: leptos::web_sys::MouseEvent| {})
            .disabled(true)
            .build();
        let html = Button(props).into_view().to_html();
        assert!(html.contains("disabled"), "{html}");
        assert!(html.contains(">Save<"), "{html}");
    }

    #[test]
    fn flag_attributes_only_appear_when_set() {
        let state = ButtonState::new(
            ButtonVariant::Solid,
            ButtonColor::Accent,
            ButtonSize::Sm,
            true,
            false,
            true,
            false,
        );
        let html = render(state, "Go");
        assert!(html.contains("data-variant=\"solid\""), "{html}");
        assert!(html.contains("data-color=\"accent\""), "{html}");
        assert!(html.contains("data-size=\"sm\""), "{html}");
        assert!(html.contains("data-full-width"), "{html}");
        assert!(html.contains("data-elevated"), "{html}");
        assert!(html.contains("t-button-elevated"), "{html}");
    }

    #[test]
    fn loading_center_hides_content_and_marks_busy() {
        let state = ButtonState::new(
            ButtonVariant::Ghost,
            ButtonColor::Danger,
            ButtonSize::Md,
            false,
            true,
            false,
            false,
        );
        let html = render(state, "Wait");
        assert!(html.contains("data-loading"), "{html}");
        assert!(html.contains("aria-busy=\"true\""), "{html}");
        assert!(html.contains(BUTTON_HIDDEN_CLASS), "{html}");
        assert!(html.contains(BUTTON_LOADER_CLASS), "{html}");
        assert!(html.contains("disabled"), "{html}");
    }

    #[test]
    fn solid_inherit_is_rejected() {
        assert_eq!(
            resolve_button(ButtonVariant::Solid, ButtonColor::Inherit),
            Err(ButtonError::InheritSolid)
        );
        assert!(resolve_button(ButtonVariant::Outline, ButtonColor::Inherit).is_ok());
    }

    #[test]
    fn variant_and_size_strings_match_upstream_enum() {
        assert_eq!(ButtonVariant::Solid.as_str(), "solid");
        assert_eq!(ButtonVariant::Outline.as_str(), "outline");
        assert_eq!(ButtonVariant::Ghost.as_str(), "ghost");
        assert_eq!(ButtonVariant::Soft.as_str(), "soft");
        assert_eq!(ButtonVariant::Link.as_str(), "link");
        assert_eq!(ButtonSize::Sm.as_str(), "sm");
        assert_eq!(ButtonSize::Md.as_str(), "md");
        for (color, want) in [
            (ButtonColor::Neutral, "neutral"),
            (ButtonColor::Accent, "accent"),
            (ButtonColor::Danger, "danger"),
            (ButtonColor::Success, "success"),
            (ButtonColor::Inherit, "inherit"),
        ] {
            assert_eq!(color.as_str(), want);
        }
    }
}
