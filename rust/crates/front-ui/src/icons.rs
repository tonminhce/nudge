use leptos::prelude::*;

#[cfg(test)]
mod tests;

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
    ICON_DEFS.iter().find(|def| def.tabler_name == name)
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
    format!("tabler-icon tabler-icon-{}", def.tabler_name)
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
    for d in def.d_values {
        out.push_str("<path d=\"");
        out.push_str(d);
        out.push_str("\"/>");
    }
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
    let body = def
        .d_values
        .iter()
        .map(|d| (*d).to_string())
        .collect::<Vec<_>>();
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
        >
            {move || {
                body.iter()
                    .map(|d| view! { <path d=d.clone() /> })
                    .collect::<Vec<_>>()
            }}
        </svg>
    }
}
