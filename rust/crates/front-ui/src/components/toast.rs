use leptos::callback::Callable;
use leptos::prelude::*;

use crate::components::button::join_classes;
use crate::components::next_dom_id;
use crate::icons::Icon;

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
            Self::Info => "info",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    pub fn is_alert(self) -> bool {
        self == Self::Error
    }

    pub fn icon_name(self) -> &'static str {
        match self {
            Self::Info => "info-circle",
            Self::Success => "square-rounded-check",
            Self::Warning => "alert-triangle",
            Self::Error => "circle-x",
        }
    }

    pub fn aria_label(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }

    pub fn icon_color(self) -> &'static str {
        match self {
            Self::Info => "color(display-p3 .276 .384 .837)",
            Self::Success => "color(display-p3 .297 .637 .581)",
            Self::Warning => "color(display-p3 .6 .45 .1)",
            Self::Error => "color(display-p3 .8 .2 .2)",
        }
    }

    pub fn countdown_color(self) -> &'static str {
        match self {
            Self::Info => "#465eff19",
            Self::Success => "#00a43319",
            Self::Warning => "#b7790019",
            Self::Error => "#d0000019",
        }
    }
}

pub const TOAST_VIEWPORT_CLASS: &str = "t-toast-viewport";
pub const TOAST_ITEM_CLASS: &str = "t-toast-item";
pub const TOAST_ITEM_CONTENT_CLASS: &str = "t-toast-item-content";
pub const TOAST_CLASS: &str = "t-toast";
pub const TOAST_PROGRESS_CLASS: &str = "t-toast-progress";
pub const TOAST_COUNTDOWN_CLASS: &str = "t-toast-countdown";
pub const TOAST_HEADER_CLASS: &str = "t-toast-header";
pub const TOAST_ICON_CLASS: &str = "t-toast-icon";
pub const TOAST_MESSAGE_CLASS: &str = "t-toast-message";
pub const TOAST_TITLE_CLASS: &str = "t-toast-title";
pub const TOAST_DESCRIPTION_CLASS: &str = "t-toast-description";
pub const TOAST_BODY_CLASS: &str = "t-toast-body";
pub const TOAST_ACTIONS_CLASS: &str = "t-toast-actions";
pub const TOAST_DISMISS_CLASS: &str = "t-toast-dismiss";
pub const TOAST_DISMISS_CONTENT_CLASS: &str = "t-toast-dismiss-content";
pub const TOAST_DISMISS_ICON_CLASS: &str = "t-toast-dismiss-icon";

#[cfg(target_arch = "wasm32")]
fn animate_toast(item: &leptos::web_sys::HtmlDivElement) {
    use js_sys::Array;
    use js_sys::Function;
    use js_sys::Object;
    use js_sys::Reflect;
    use leptos::wasm_bindgen::JsValue;

    let frame = |property: &str, value: &str| {
        let frame = Object::new();
        Reflect::set(
            &frame,
            &JsValue::from_str(property),
            &JsValue::from_str(value),
        )
        .expect("toast animation frame");
        frame
    };
    let animate = |property: &str, from: &str, to: &str| {
        let frames = Array::new();
        frames.push(&frame(property, from));
        frames.push(&frame(property, to));
        let timing = Object::new();
        Reflect::set(
            &timing,
            &JsValue::from_str("duration"),
            &JsValue::from_f64(500.0),
        )
        .expect("toast animation duration");
        Reflect::set(
            &timing,
            &JsValue::from_str("easing"),
            &JsValue::from_str("ease-in-out"),
        )
        .expect("toast animation easing");
        Reflect::set(
            &timing,
            &JsValue::from_str("fill"),
            &JsValue::from_str("backwards"),
        )
        .expect("toast animation fill");
        let animate_function = Reflect::get(item.as_ref(), &JsValue::from_str("animate"))
            .expect("toast animate function");
        let animate_function = Function::from(animate_function);
        animate_function
            .call2(item.as_ref(), &frames.into(), &timing.into())
            .expect("toast animation");
    };
    animate("gridTemplateRows", "0.0625px", "0.0625px");
    animate("opacity", "0", "1");
    animate("transform", "translateY(40px)", "translateY(0px)");
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn animate_toast(_item: &leptos::web_sys::HtmlDivElement) {}

#[component]
pub fn ToastViewport(children: Children) -> impl IntoView {
    view! {
        <div
            class=TOAST_VIEWPORT_CLASS
            role="region"
            dir="ltr"
            aria-label="Notifications"
            data-globally-prevent-click-outside="true"
            style="z-index: 10002;"
        >
            {children()}
        </div>
    }
}

#[component]
pub fn Toast(
    #[prop(into)] title: String,
    #[prop(optional)] description: &'static str,
    #[prop(optional)] children: Option<Children>,
    #[prop(optional)] tone: Option<ToastTone>,
    #[prop(optional, into)] on_dismiss: Option<Callback<()>>,
    #[prop(optional)] duration: Option<u32>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let item_ref = NodeRef::new();
    #[cfg(target_arch = "wasm32")]
    request_animation_frame(move || {
        if let Some(item) = item_ref.get() {
            animate_toast(&item);
        }
    });
    let tone = tone.unwrap_or_default();
    let root_class = join_classes(&[
        Some(TOAST_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let item_class = join_classes(&[
        Some(TOAST_ITEM_CLASS),
        if class.is_empty() { None } else { Some(class) },
    ]);
    let children_slot = children.map(|children| children().into_any());
    let has_details = !description.is_empty() || children_slot.is_some();
    let message_content = if has_details {
        view! {
            <div class=TOAST_TITLE_CLASS>{title.clone()}</div>
            {if description.is_empty() {
                None
            } else {
                Some(view! { <div class=TOAST_DESCRIPTION_CLASS>{description}</div> })
            }}
            {children_slot.map(|node| view! { <div class=TOAST_BODY_CLASS>{node}</div> })}
        }
        .into_any()
    } else {
        view! { {title.clone()} }.into_any()
    };
    let duration_value = duration;
    let countdown_duration = duration.unwrap_or(6000);
    let toast_id = next_dom_id("toast");
    #[cfg(target_arch = "wasm32")]
    if let (Some(duration), Some(on_dismiss)) = (duration, on_dismiss) {
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(duration).await;
            on_dismiss.run(());
        });
    }
    view! {
        <div class=item_class node_ref=item_ref>
            <div class=TOAST_ITEM_CONTENT_CLASS>
            <div
                class=root_class
                role=if tone.is_alert() { "alert" } else { "status" }
                aria-live="polite"
                id=toast_id
                title=title.clone()
                data-tone=tone.as_str()
                data-duration=duration_value.map(|duration| duration.to_string())
            >
                <div class=TOAST_PROGRESS_CLASS aria-hidden="true">
                    <div
                        class=TOAST_COUNTDOWN_CLASS
                        style=format!(
                            "--toast-countdown-color: {}; --toast-countdown-width: 100%; --toast-countdown-duration: {}ms;",
                            tone.countdown_color(),
                            countdown_duration
                        )
                    ></div>
                </div>
                <div class=TOAST_HEADER_CLASS>
                    <div class=TOAST_ICON_CLASS>
                        <Icon name=tone.icon_name() size=16 color=tone.icon_color() aria_label=tone.aria_label()/>
                    </div>
                    <div class=TOAST_MESSAGE_CLASS>{message_content}</div>
                    {on_dismiss.map(|_| view! {
                        <div class=TOAST_ACTIONS_CLASS>
                            <button
                                type="button"
                                tabindex="0"
                                class=TOAST_DISMISS_CLASS
                                title="Close"
                                aria-label="Close"
                                on:click=move |_| {
                                    if let Some(on_dismiss) = on_dismiss {
                                        on_dismiss.run(());
                                    }
                                }
                            >
                                <span class=TOAST_DISMISS_CONTENT_CLASS>
                                    <span class=TOAST_DISMISS_ICON_CLASS aria-hidden="true">
                                        <Icon name="x" size=24/>
                                    </span>
                                </span>
                            </button>
                        </div>
                    })}
                </div>
            </div>
            </div>
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
        assert!(out.contains("aria-live=\"polite\""), "{out}");
        assert!(out.contains("data-tone=\"info\""), "{out}");
        assert!(out.contains(TOAST_TITLE_CLASS), "{out}");
        assert!(out.contains(">Saved<"), "{out}");
        assert!(out.contains(TOAST_DESCRIPTION_CLASS), "{out}");
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
        assert!(out.contains("role=\"alert\""), "{out}");
        assert!(out.contains(TOAST_BODY_CLASS), "{out}");
        assert!(out.contains(">Try again.<"), "{out}");
        assert!(out.contains("upload-toast"), "{out}");
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
        assert!(out.contains(TOAST_COUNTDOWN_CLASS), "{out}");
        assert!(out.contains("--toast-countdown-duration: 5000ms"), "{out}");
        assert!(out.contains("aria-label=\"Close\""), "{out}");
        assert!(out.contains("tabler-icon-square-rounded-check"), "{out}");
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

    #[test]
    fn viewport_uses_the_upstream_notifications_region() {
        let out = (view! {
            <ToastViewport>
                <Toast title="Saved"/>
            </ToastViewport>
        })
        .into_view()
        .to_html();
        assert!(out.contains("class=\"t-toast-viewport\""), "{out}");
        assert!(out.contains("role=\"region\""), "{out}");
        assert!(out.contains("aria-label=\"Notifications\""), "{out}");
        assert!(
            out.contains("data-globally-prevent-click-outside=\"true\""),
            "{out}"
        );
    }
}
