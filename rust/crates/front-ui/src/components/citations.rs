pub const CLASS_CONTRACTS: &[(&str, &str)] = &[
    (
        "Button",
        "packages/twenty-ui/src/primitives/input/Button/Button.module.scss (classes: button, content, label, icon, hidden, loader; data-variant/data-color/data-size/data-full-width/data-loading/data-elevated/data-disabled at Button.tsx:66-72)",
    ),
    (
        "Chip",
        "packages/twenty-ui/src/primitives/data-display/Chip/Chip.module.scss (classes: chip, label, endElementDivider; data-size/data-shape/data-color/data-weight/data-variant/data-truncate at Chip.tsx:47-91)",
    ),
    (
        "Tag",
        "packages/twenty-ui/src/primitives/data-display/Tag/Tag.module.scss (classes: tag, content, overflowingText, iconContainer; --tw-tag-background/--tw-tag-text/--tw-tag-border-style at Tag.tsx:39-51)",
    ),
    (
        "Input",
        "packages/twenty-ui/src/primitives/input/Input/Input.module.scss (classes: input, sm, md; data-variant/data-focus-ring/data-grouped at Input.tsx:29-35)",
    ),
    (
        "InputGroup",
        "packages/twenty-ui/src/primitives/input/InputGroup/InputGroup.module.scss (classes: root, sm, md, startElement, endElement)",
    ),
    (
        "Select",
        "packages/twenty-ui/src/primitives/input/Select/Select.module.scss (classes: trigger, value, icon, positioner, popup, item, indicator, groupLabel, separator; trigger composition at internal/SelectTrigger.tsx:18)",
    ),
    (
        "Dialog",
        "packages/twenty-ui/src/primitives/surfaces/internal/Dialog.module.scss (classes: backdrop, viewport, popup, header, body, footer, title, description; data-size at internal/DialogPopup.tsx:20)",
    ),
    (
        "Tooltip",
        "packages/twenty-ui/src/primitives/surfaces/Tooltip/Tooltip.module.scss (classes: positioner, popup, content, title, icon, description, arrow)",
    ),
    (
        "Tabs",
        "packages/twenty-ui/src/primitives/navigation/internal/tab/Tab.module.scss (classes: root, list, tab, content, label, startIcon, badge, panel, indicator; data-size/data-variant/data-highlighted at TabsTab.tsx:22-25, data-active at components/navigation/TabButton/TabButton.tsx:31)",
    ),
    (
        "Avatar",
        "packages/twenty-ui/src/primitives/data-display/Avatar/Avatar.module.scss (classes: root, image, fallback; data-size/data-shape/data-pulsing/data-ring at internal/AvatarRoot.tsx:60-63; --tw-avatar-color/--tw-avatar-background/--tw-avatar-border at AvatarRoot.tsx:66-77)",
    ),
    (
        "IconButton",
        "packages/twenty-ui/src/components/input/IconButton/IconButton.module.scss (classes: button, tooltipAnchor, icon; data-icon-size/data-shape at IconButton.tsx:33-39)",
    ),
    (
        "Menu",
        "packages/twenty-ui/src/components/navigation/Dropdown/Dropdown.module.scss (classes: header, content, section, group, groupLabel, separator, itemWithActions; option rows reuse primitives/navigation/ListItem/ListItem.module.scss root, label, startIcon, endIcon, hotkeys)",
    ),
    (
        "SidePanel",
        "packages/twenty-front/src/modules/side-panel/components/SidePanelForDesktop.tsx:29-64 (StyledSidePanelWrapper/StyledSidePanel aside, width var, data-shrink-from-full-width)",
    ),
    (
        "RecordTable",
        "packages/twenty-front/src/modules/object-record/record-table/constants/RecordTableRowHeight.ts:1 (32px rows), RecordTableColumnMinWidth.ts:1 (104px min width), virtualization/constants/RowSlotCount.ts:1 (240 slots)",
    ),
];

pub const TOKEN_SHEET_CONTRACT: &str =
    "packages/twenty-ui/src/theme/theme-light.css / theme-dark.css (@layer twenty-ui, .light/.dark class scoping, @supports (corner-shape: squircle) block doubling radii)";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_component_module_has_a_cited_contract() {
        let covered = [
            "Button",
            "Chip",
            "Tag",
            "Input",
            "InputGroup",
            "Select",
            "Dialog",
            "Tooltip",
            "Tabs",
            "Avatar",
            "IconButton",
            "Menu",
            "SidePanel",
            "RecordTable",
        ];
        for name in covered {
            assert!(
                CLASS_CONTRACTS
                    .iter()
                    .any(|(component, _)| *component == name),
                "{name} missing from the class-contract citation map"
            );
        }
    }

    #[test]
    fn citations_point_at_upstream_paths() {
        for (name, path) in CLASS_CONTRACTS {
            assert!(
                path.starts_with("packages/twenty-ui/src/")
                    || path.starts_with("packages/twenty-front/src/"),
                "{name}: citation must be a twenty-ui or twenty-front path: {path}"
            );
        }
        assert!(TOKEN_SHEET_CONTRACT.contains("theme-light.css"));
        assert!(TOKEN_SHEET_CONTRACT.contains("theme-dark.css"));
    }
}
