use leptos::prelude::*;

#[cfg(test)]
mod tests;

mod custom;
pub use custom::*;

include!("icons/generated.rs");

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum IconStroke {
    Sm,
    #[default]
    Md,
    Lg,
}

impl IconStroke {
    pub fn value(self) -> &'static str {
        match self {
            IconStroke::Sm => "1.6",
            IconStroke::Md => "2",
            IconStroke::Lg => "2.5",
        }
    }
}

pub fn icon_def(name: &str) -> Option<&'static IconDef> {
    ICON_DEFS
        .binary_search_by(|def| def.tabler_name.cmp(name))
        .ok()
        .map(|index| &ICON_DEFS[index])
}

pub const OBJECT_ICONS: &[(&str, &str)] = &[
    ("person", "user"),
    ("company", "building-skyscraper"),
    ("opportunity", "target-arrow"),
    ("task", "checkbox"),
    ("note", "notes"),
    ("view", "table"),
    ("workflow", "settings-automation"),
    ("workflows", "settings-automation"),
    ("agent", "lego"),
    ("agents", "lego"),
];

pub fn object_icon(key: &str) -> Option<&'static str> {
    OBJECT_ICONS
        .iter()
        .find(|(k, _)| *k == key)
        .map(|&(_, tabler)| tabler)
}

pub fn icon_defs() -> &'static [IconDef] {
    ICON_DEFS
}

pub fn icon_class(def: &IconDef) -> String {
    def.class_name.to_string()
}

pub fn render_icon(def: &IconDef, size: u16, stroke: IconStroke, color: &str) -> String {
    let width = size.to_string();
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{width}\" \
         viewBox=\"{vb}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{sw}\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\" class=\"{cls}\">",
        vb = def.view_box,
        sw = stroke.value(),
        cls = icon_class(def),
    );
    out.push_str(def.inner_svg);
    out.push_str("</svg>");
    out
}

#[component]
pub fn Icon(
    name: &'static str,
    #[prop(optional)] size: u16,
    #[prop(optional)] stroke: Option<IconStroke>,
    #[prop(optional)] color: &'static str,
) -> impl IntoView {
    let def = icon_def(name).unwrap_or(&ICON_DEFS[0]);
    let size = if size == 0 { 16 } else { size };
    let stroke = stroke.unwrap_or(IconStroke::Md);
    let color = if color.is_empty() {
        "currentColor"
    } else {
        color
    };
    let width = size.to_string();
    let view_box = def.view_box.to_string();
    let class = icon_class(def);
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width=width.clone()
            height=width
            viewBox=view_box
            fill="none"
            stroke=color
            stroke-width=stroke.value()
            stroke-linecap="round"
            stroke-linejoin="round"
            class=class
            inner_html=def.inner_svg
        />
    }
}

#[component]
pub fn IconSparkle2(
    #[prop(optional)] size: u16,
    #[prop(optional)] stroke: Option<IconStroke>,
    #[prop(optional)] color: &'static str,
) -> impl IntoView {
    let size = if size == 0 { 24 } else { size };
    let stroke = stroke.unwrap_or(IconStroke::Md);
    let color = if color.is_empty() {
        "currentColor"
    } else {
        color
    };
    let width = size.to_string();
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width=width.clone()
            height=width
            viewBox="0 0 24 24"
            fill="none"
            stroke=color
            stroke-width=stroke.value()
            stroke-linecap="round"
            stroke-linejoin="round"
            class="tabler-icon tabler-icon-sparkle-2"
            inner_html=r#"<path d="M12 3c.375 0 .711 .231 .846 .581l1.65 4.29a2.85 2.85 0 0 0 1.632 1.633l4.291 1.65a.906 .906 0 0 1 0 1.692l-4.29 1.65a2.84 2.84 0 0 0 -1.633 1.632l-1.65 4.291a.906 .906 0 0 1 -1.692 0l-1.65 -4.29a2.84 2.84 0 0 0 -1.632 -1.633l-4.291 -1.65a.906 .906 0 0 1 0 -1.692l4.29 -1.65a2.84 2.84 0 0 0 1.633 -1.632l1.65 -4.291a.91 .91 0 0 1 .846 -.581"></path>"#
        />
    }
}

#[component]
pub fn ThinkingOrbitLoaderIcon(
    #[prop(optional)] size: u16,
    #[prop(optional)] color: &'static str,
) -> impl IntoView {
    view! { <IconAnimatedLoader size=size color=color /> }
}

#[component]
pub fn IconChartBarHorizontal(
    #[prop(optional)] size: u16,
    #[prop(optional)] stroke: Option<IconStroke>,
    #[prop(optional)] color: &'static str,
) -> impl IntoView {
    view! {
        <span style="display:inline-flex;flex-shrink:0;transform:rotate(90deg)">
            <IconChartBar size=size stroke=stroke.unwrap_or_default() color=color />
        </span>
    }
}

#[component]
pub fn IconAnimatedLoader(
    #[prop(optional)] size: u16,
    #[prop(optional)] color: &'static str,
) -> impl IntoView {
    let size = if size == 0 { 14 } else { size };
    let color = if color.is_empty() {
        "currentColor"
    } else {
        color
    };
    let width = size.to_string();
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width=width.clone()
            height=width
            viewBox="0 0 14 14"
            fill="none"
            stroke=color
            aria-hidden="true"
            focusable="false"
            class="t-icon-animated-loader"
            inner_html=r#"<path d="M3.1 7 C3.1 4.4 6.0 4.4 7.0 7 C8.0 9.6 10.9 9.6 10.9 7 C10.9 4.4 8.0 4.4 7.0 7 C6.0 9.6 3.1 9.6 3.1 7" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" shape-rendering="geometricPrecision" pathLength="100" stroke-dasharray="14 86" stroke-dashoffset="0"><animate attributeName="stroke-dashoffset" values="0;-100" dur="1.05s" repeatCount="indefinite"></animate><animate attributeName="stroke-dasharray" values="10 90;16 84;10 90" dur="1.05s" repeatCount="indefinite"></animate></path>"#
        />
    }
}
