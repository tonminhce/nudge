use leptos::prelude::*;

use crate::components::{
    AppHeader, Avatar, AvatarSize, AvatarVariant, Button, ButtonColor, ButtonSize, ButtonVariant,
    Chip, ChipColor, ChipShape, ChipSize, ChipVariant, IconButton, Input, InputSize, Menu,
    MenuItemDef, Modal, ModalSize, NavigationDrawer, RecordTable, Select, SelectItem, SidePanel,
    TabItem, TabList, TabPanel, TableColumn, Tag, TagVariant, ThemeScope, Tooltip, TooltipSide,
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
                <section class="gallery-cell">
                    <h3>{"Table row skeleton"}</h3>
                    <RecordTable columns=cols.clone() total_rows=1000 checkbox_column=true/>
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
        assert!(html.contains(">Table panel<"), "tab panel");
        assert!(html.contains("t-avatar-fallback\">T<"), "avatar initial");
        assert!(html.contains(">Duplicate<"), "menu item");
        assert!(html.contains(">Create view<"), "modal title");
        assert!(html.contains("Side panel content"), "side panel");
        assert!(html.contains("Keyboard shortcut: Ctrl+S"), "tooltip");
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
