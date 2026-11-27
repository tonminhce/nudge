use std::collections::BTreeSet;

use leptos::prelude::*;

use crate::components::table::display_value;
use crate::components::{
    AppHeader, Avatar, AvatarSize, AvatarVariant, Badge, BadgeTone, BadgeVariant, Breadcrumb,
    BreadcrumbItem, Button, ButtonColor, ButtonSize, ButtonVariant, Checkbox, Chip, ChipColor,
    ChipShape, ChipSize, ChipVariant, Collapsible, IconButton, Input, InputSize, ListItem, Menu,
    MenuItemDef, Modal, ModalSize, NavigationDrawer, Popover, PopoverPlacement, RecordTable,
    Select, SelectItem, Separator, SidePanel, Switch, TabItem, TabList, TabPanel, TableColumn,
    TableRow, Tag, TagVariant, Textarea, ThemeScope, Toast, ToastTone, Tooltip, TooltipSide,
};
use crate::icons::{Icon, IconStroke};
use crate::tokens::Scheme;

fn demo_columns() -> Vec<TableColumn> {
    vec![
        TableColumn::new("name", "Name"),
        TableColumn::new("email", "Email"),
        TableColumn::new("company", "Company"),
    ]
}

fn demo_rows() -> Vec<TableRow> {
    vec![
        TableRow::new(
            "person-1",
            vec![
                leptos::serde_json::json!("Ada Lovelace"),
                leptos::serde_json::json!("ada@example.com"),
                leptos::serde_json::json!("Analytical Engines"),
            ],
        ),
        TableRow::new(
            "person-2",
            vec![
                leptos::serde_json::json!("Grace Hopper"),
                leptos::serde_json::json!("grace@example.com"),
                leptos::serde_json::json!("Compilers Inc"),
            ],
        ),
        TableRow::new(
            "person-3",
            vec![
                leptos::serde_json::json!("Katherine Johnson"),
                leptos::serde_json::json!("katherine@example.com"),
                leptos::serde_json::json!("Orbital Research"),
            ],
        ),
        TableRow::new(
            "person-4",
            vec![
                leptos::serde_json::json!("Margaret Hamilton"),
                leptos::serde_json::json!("margaret@example.com"),
                leptos::serde_json::json!("Flight Software"),
            ],
        ),
    ]
}

fn demo_select_items() -> Vec<SelectItem> {
    vec![
        SelectItem::new("Open"),
        SelectItem::selected("In progress"),
        SelectItem {
            label: "Closed".to_string(),
            selected: false,
            disabled: false,
        },
    ]
}

fn demo_menu_items() -> Vec<MenuItemDef> {
    vec![
        MenuItemDef::new("Edit"),
        MenuItemDef::new("Duplicate"),
        MenuItemDef {
            label: "Delete".to_string(),
            selected: false,
            disabled: false,
            accent: "danger",
        },
    ]
}

pub fn gallery_section(scheme: Scheme) -> impl IntoView {
    let cols = demo_columns();
    let select_items = demo_select_items();
    let menu_items = demo_menu_items();
    let (popover_open, set_popover_open) = signal(false);
    let (details_open, set_details_open) = signal(true);
    let (toast_visible, set_toast_visible) = signal(true);
    let selected_ids = RwSignal::new(BTreeSet::new());
    let sort_column = RwSignal::new(String::from("name"));
    let sort_ascending = RwSignal::new(true);
    let last_action = RwSignal::new(String::from("Try sorting, selection, or row activation"));
    let sorted_rows = move || {
        let mut rows = demo_rows();
        let column = sort_column.get();
        let column_index = ["name", "email", "company"]
            .iter()
            .position(|key| *key == column)
            .unwrap_or(0);
        let ascending = sort_ascending.get();
        rows.sort_by(|left, right| {
            let ordering = left.cells[column_index]
                .to_string()
                .cmp(&right.cells[column_index].to_string());
            if ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
        rows
    };
    let on_sort = move |key: String| {
        if sort_column.get() == key {
            sort_ascending.update(|ascending| *ascending = !*ascending);
        } else {
            sort_column.set(key.clone());
            sort_ascending.set(true);
        }
        last_action.set(format!("Sorted by {key}"));
    };
    let on_select = move |(id, selected): (String, bool)| {
        if selected {
            selected_ids.update(|ids| {
                ids.insert(id.clone());
            });
        } else {
            selected_ids.update(|ids| {
                ids.remove(&id);
            });
        }
        last_action.set(format!("{id} selected: {selected}"));
    };
    let on_row_click = move |id: String| last_action.set(format!("Opened {id}"));
    let render_cell = move |(_, column, value): (usize, usize, leptos::serde_json::Value)| {
        if column == 0 {
            view! { <span class="gallery-record-name">{display_value(&value)}</span> }.into_any()
        } else if column == 2 {
            view! { <span class="gallery-company-badge">{display_value(&value)}</span> }.into_any()
        } else {
            view! { <span>{display_value(&value)}</span> }.into_any()
        }
    };
    view! {
        <ThemeScope scheme=scheme class="gallery">
            <AppHeader><strong>{"Nudge front-ui"}</strong></AppHeader>
            <div class="gallery-grid">
                <section class="gallery-cell">
                    <h3>{"Buttons"}</h3>
                    <div class="gallery-row">
                        <Button variant=ButtonVariant::Solid color=ButtonColor::Accent size=ButtonSize::Sm>{"Accent solid"}</Button>
                        <Button variant=ButtonVariant::Outline>{"Neutral outline"}</Button>
                        <Button variant=ButtonVariant::Soft>{"Soft"}</Button>
                        <Button variant=ButtonVariant::Ghost>{"Ghost"}</Button>
                        <Button variant=ButtonVariant::Link>{"Link"}</Button>
                        <Button color=ButtonColor::Danger variant=ButtonVariant::Solid>{"Danger"}</Button>
                        <Button color=ButtonColor::Success>{"Success"}</Button>
                    </div>
                </section>
                <section class="gallery-cell">
                    <h3>{"Icon buttons & icons"}</h3>
                    <div class="gallery-row">
                        <IconButton><Icon name="search" stroke=IconStroke::Sm/></IconButton>
                        <IconButton tooltip="Add"><Icon name="plus"/></IconButton>
                        <Icon name="user"/>
                        <Icon name="building-skyscraper"/>
                        <Icon name="mail"/>
                        <Icon name="lock"/>
                        <Icon name="settings"/>
                        <Icon name="calendar"/>
                        <Icon name="table"/>
                        <Icon name="filter"/>
                        <Icon name="bell"/>
                    </div>
                </section>
                <section class="gallery-cell">
                    <h3>{"Chips & tags"}</h3>
                    <div class="gallery-row">
                        <Chip variant=ChipVariant::Solid shape=ChipShape::Round size=ChipSize::Md>{"Solid chip"}</Chip>
                        <Chip variant=ChipVariant::Soft color=ChipColor::Secondary>{"Soft chip"}</Chip>
                        <Chip variant=ChipVariant::Outline>{"Outline chip"}</Chip>
                        <Tag>{"Gray tag"}</Tag>
                        <Tag color="blue" variant=TagVariant::Outline>{"Blue tag"}</Tag>
                        <Tag color="red">{"Red tag"}</Tag>
                        <Tag weight=crate::components::TagWeight::Medium>{"Medium"}</Tag>
                    </div>
                </section>
                <section class="gallery-cell">
                    <h3>{"Inputs & select"}</h3>
                    <div class="gallery-row">
                        <Input placeholder="Text input"/>
                        <Input placeholder="Small input" size=InputSize::Sm/>
                        <Select placeholder="Pick a status" items=select_items.clone() group_label="Status"/>
                    </div>
                </section>
                <section class="gallery-cell">
                    <h3>{"Forms & status"}</h3>
                    <div class="gallery-row">
                        <Textarea value="Notes" placeholder="Add notes" rows=3/>
                        <Checkbox checked=true label="Subscribe"/>
                        <Switch checked=true label="Notifications"/>
                        <Badge label="New" tone=BadgeTone::Accent variant=BadgeVariant::Soft/>
                        <Badge label="Open" tone=BadgeTone::Success variant=BadgeVariant::Outline/>
                        <Badge label="Review" tone=BadgeTone::Warning variant=BadgeVariant::Solid/>
                        <Separator/>
                    </div>
                </section>
                <section class="gallery-cell">
                    <h3>{"Tabs"}</h3>
                    <div class="gallery-row">
                        <TabList tabs=vec![TabItem::new("table","Table"), TabItem::new("kanban","Kanban"), TabItem::new("calendar","Calendar")] value="table"/>
                    </div>
                    <TabPanel value="table">{"Table panel"}</TabPanel>
                </section>
                <section class="gallery-cell">
                    <h3>{"Avatars"}</h3>
                    <div class="gallery-row">
                        <Avatar name="tony" size=AvatarSize::Lg shape=crate::components::AvatarShape::Circle/>
                        <Avatar name="Alice"/>
                        <Avatar name="Bob" variant=AvatarVariant::Outline/>
                        <Avatar name="" size=AvatarSize::Sm/>
                    </div>
                </section>
                <section class="gallery-cell">
                    <h3>{"Menu & tooltip"}</h3>
                    <div class="gallery-row">
                        <Menu items=menu_items.clone() group_label="Actions" open=true/>
                        <Tooltip content="Keyboard shortcut: Ctrl+S" side=TooltipSide::Top open=true/>
                    </div>
                </section>
                <section class="gallery-cell">
                    <h3>{"Modal & side panel"}</h3>
                    <div class="gallery-row">
                        <Modal title="Create view" size=ModalSize::Md open=true description="Choose a view type">{"Modal body"}</Modal>
                        <SidePanel open=true width=400u32>{"Side panel content"}</SidePanel>
                    </div>
                </section>
                <section class="gallery-cell gallery-cell-wide">
                    <h3>{"Records table"}</h3>
                    <Breadcrumb items=vec![
                        BreadcrumbItem::new("Workspace", "/"),
                        BreadcrumbItem::new("People", "/object/people"),
                        BreadcrumbItem::current("All people"),
                    ]/>
                    <RecordTable
                        columns=cols.clone()
                        total_rows=4
                        data=sorted_rows()
                        checkbox_column=true
                        selected_ids=selected_ids.get()
                        on_select=on_select
                        on_row_click=on_row_click
                        on_sort=on_sort
                        sort_column=sort_column.get()
                        sort_ascending=sort_ascending.get()
                        render_cell=render_cell
                    />
                    <p class="gallery-action-status" aria-live="polite">{last_action.get()}</p>
                </section>
                <section class="gallery-cell">
                    <h3>{"Navigation & feedback"}</h3>
                    <div class="gallery-row">
                        <Popover
                            open=popover_open
                            on_open_change=move |next| set_popover_open.set(next)
                            placement=PopoverPlacement::BottomStart
                            trigger=view! { "Actions" }.into_any()
                            content=view! {
                                <div class="gallery-popover-menu">
                                    <ListItem title="Profile settings" href="/settings"/>
                                    <ListItem title="Notification preferences" href="/settings"/>
                                </div>
                            }.into_any()
                        />
                        <Collapsible
                            open=details_open
                            on_open_change=move |next| set_details_open.set(next)
                            trigger=view! { "Details" }.into_any()
                            content=view! { "Collapsible record details" }.into_any()
                        />
                    </div>
                    <Show when=move || toast_visible.get() fallback=|| ()>
                        <Toast
                            title="Changes saved"
                            description="The record is up to date."
                            tone=ToastTone::Success
                            duration=5000
                            on_dismiss=move |_| set_toast_visible.set(false)
                        />
                    </Show>
                </section>
                <section class="gallery-cell">
                    <h3>{"Table row skeleton"}</h3>
                    <RecordTable columns=cols.clone() total_rows=10 checkbox_column=true/>
                    <NavigationDrawer collapsed=true/>
                </section>
            </div>
        </ThemeScope>
    }
}

pub fn gallery_page() -> impl IntoView {
    view! {
        <main class="gallery-page">
            {gallery_section(Scheme::Light)}
            {gallery_section(Scheme::Dark)}
        </main>
    }
}

pub fn render_gallery() -> String {
    gallery_page().into_view().to_html()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_renders_all_primitives_in_light_and_dark() {
        let html = render_gallery();
        assert_eq!(
            html.matches("class=\"light gallery\"").count(),
            1,
            "exactly one light scope"
        );
        assert_eq!(
            html.matches("class=\"dark gallery\"").count(),
            1,
            "exactly one dark scope"
        );
        assert!(html.contains("Nudge front-ui"), "app header");
        assert!(html.contains("Buttons"), "button section");
        assert!(html.contains(">Accent solid<"), "button");
        assert!(html.contains(">Soft chip<"), "chip");
        assert!(html.contains(">Blue tag<"), "tag");
        assert!(html.contains("Pick a status"), "select");
        assert!(html.contains(">Notes</textarea>"), "textarea");
        assert!(html.contains(">Subscribe<"), "checkbox");
        assert!(html.contains("role=\"switch\""), "switch");
        assert!(html.contains(">New<"), "badge");
        assert!(
            html.contains("data-orientation=\"horizontal\""),
            "separator"
        );
        assert!(html.contains(">Table panel<"), "tab panel");
        assert!(html.contains("t-avatar-fallback\">T<"), "avatar initial");
        assert!(html.contains(">Duplicate<"), "menu item");
        assert!(html.contains(">Create view<"), "modal title");
        assert!(html.contains("Side panel content"), "side panel");
        assert!(html.contains("Keyboard shortcut: Ctrl+S"), "tooltip");
        assert!(html.contains("aria-label=\"Breadcrumb\""), "breadcrumb");
        assert!(html.contains(">All people<"), "current breadcrumb");
        assert!(html.contains(">Actions<"), "popover trigger");
        assert!(html.contains("Profile settings"), "popover item");
        assert!(
            html.contains("Collapsible record details"),
            "collapsible body"
        );
        assert!(html.contains("Changes saved"), "toast");
        assert!(
            html.contains("aria-live=\"polite\""),
            "table action feedback"
        );
    }

    #[test]
    fn gallery_renders_icons_inline() {
        let html = render_gallery();
        assert!(html.contains("tabler-icon-search"), "search icon");
        assert!(
            html.contains("tabler-icon-building-skyscraper"),
            "company icon"
        );
        assert!(html.contains("tabler-icon-mail"), "mail icon");
        assert!(html.contains("tabler-icon-bell"), "bell icon");
        assert!(html.matches("<svg").count() >= 10, "icons rendered inline");
    }

    #[test]
    fn gallery_table_emits_slot_rows() {
        let html = render_gallery();
        assert!(html.contains("data-row-slot"), "slot-indexed rows");
        assert!(html.contains(">Name<"), "table header label");
        assert!(html.contains(">Email<"), "second column");
        assert_eq!(html.matches("data-row-id=\"person-1\"").count(), 2);
        assert!(
            html.contains("gallery-record-name\">Ada Lovelace"),
            "custom name cell"
        );
        assert!(html.contains("class=\"t-table-sort\""), "sortable headers");
        assert!(html.contains("Select row 0"), "selection controls");
    }

    #[test]
    fn gallery_carries_no_comments() {
        let html = render_gallery();
        assert!(
            !html.contains("<!--"),
            "R-001: no HTML comments in rendered gallery"
        );
    }

    #[test]
    fn theme_classes_match_generated_sheets() {
        let light_sheet = std::fs::read_to_string(format!(
            "{}/theme/generated/tokens-light.css",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let dark_sheet = std::fs::read_to_string(format!(
            "{}/theme/generated/tokens-dark.css",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        assert!(light_sheet.starts_with("@layer twenty-ui {\n  .light {"));
        assert!(dark_sheet.starts_with("@layer twenty-ui {\n  .dark {"));
        let html = render_gallery();
        assert!(
            html.contains("class=\"light gallery\""),
            "light scope class emitted"
        );
        assert!(
            html.contains("class=\"dark gallery\""),
            "dark scope class emitted"
        );
    }
}
