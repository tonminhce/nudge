use leptos::prelude::*;

use crate::components::button::join_classes;

pub const ROW_SLOT_COUNT: usize = 240;
pub const RECORDS_PER_PAGE: usize = 10;
pub const OVERSCAN_PAGES: usize = 7;
pub const ROW_HEIGHT: usize = 32;
pub const COLUMN_MIN_WIDTH: usize = 104;
pub const CHECKBOX_COLUMN_WIDTH: usize = 32;
pub const TABLE_HORIZONTAL_CELL_MARGIN: usize = 8;
pub const TABLE_HORIZONTAL_CELL_PADDING: usize = 8;

pub const TABLE_CLASS: &str = "t-table";
pub const TABLE_HEAD_CLASS: &str = "t-table-head";
pub const TABLE_BODY_CLASS: &str = "t-table-body";
pub const TABLE_ROW_CLASS: &str = "t-table-row";
pub const TABLE_CELL_CLASS: &str = "t-table-cell";
pub const TABLE_CHECKBOX_CELL_CLASS: &str = "t-table-cell-checkbox";
pub const TABLE_HEADER_CELL_CLASS: &str = "t-table-headerCell";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SlotWindow {
    pub first_row_index: usize,
    pub slot_count: usize,
}

pub fn overscan_rows() -> usize {
    OVERSCAN_PAGES * RECORDS_PER_PAGE
}

pub fn allocate_row_slots(scroll_top: usize, total_rows: usize) -> SlotWindow {
    let visible = ROW_SLOT_COUNT;
    let buffer = overscan_rows();
    let scroll_rows = scroll_top / ROW_HEIGHT.max(1);
    let first = scroll_rows.saturating_sub(buffer);
    let last_possible = total_rows.saturating_sub(1);
    let first = first.min(last_possible);
    let slot_count = visible.min(total_rows.saturating_sub(first));
    SlotWindow {
        first_row_index: first,
        slot_count,
    }
}

pub fn row_offset_y(row_index: usize) -> usize {
    row_index * ROW_HEIGHT
}

pub fn total_content_height(total_rows: usize) -> usize {
    total_rows * ROW_HEIGHT
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TableColumn {
    pub key: String,
    pub label: String,
    pub width: usize,
}

impl TableColumn {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
            width: COLUMN_MIN_WIDTH,
        }
    }
}

#[component]
pub fn TableHeaderCell(column: TableColumn) -> impl IntoView {
    let width = column.width.max(COLUMN_MIN_WIDTH);
    let cls = join_classes(&[Some(TABLE_HEADER_CELL_CLASS), Some(TABLE_CELL_CLASS)]);
    view! {
        <th
            class=cls
            style=format!(
                "min-inline-size: {width}px; margin-inline: {TABLE_HORIZONTAL_CELL_MARGIN}px; \
                 padding-inline: {TABLE_HORIZONTAL_CELL_PADDING}px;"
            )
            data-column-key=column.key.clone()
        >{column.label}</th>
    }
}

#[component]
pub fn TableRowSkeleton(
    row_index: usize,
    columns: Vec<TableColumn>,
    #[prop(optional)] selected: bool,
    #[prop(optional)] checkbox: bool,
) -> impl IntoView {
    let height = ROW_HEIGHT;
    let y = row_offset_y(row_index);
    let row_class = join_classes(&[
        Some(TABLE_ROW_CLASS),
        selected.then_some("t-table-row-selected"),
    ]);
    let cells: Vec<_> = columns
        .iter()
        .map(|column| {
            let key = column.key.clone();
            view! {
                <td
                    class=TABLE_CELL_CLASS
                    data-column-key=key
                    style=format!(
                        "block-size: {height}px; min-inline-size: {}px; \
                         padding-inline: {TABLE_HORIZONTAL_CELL_PADDING}px;",
                        column.width.max(COLUMN_MIN_WIDTH)
                    )
                ></td>
            }
        })
        .collect();
    let checkbox_cell = checkbox.then(|| {
        view! {
            <td
                class=join_classes(&[
                    Some(TABLE_CHECKBOX_CELL_CLASS),
                    Some(TABLE_CELL_CLASS),
                ])
                style=format!(
                    "inline-size: {CHECKBOX_COLUMN_WIDTH}px; block-size: {height}px;"
                )
                data-checkbox-cell=""
            >
                <input type="checkbox" aria-label=format!("Select row {row_index}") />
            </td>
        }
    });
    view! {
        <tr
            class=row_class
            data-row-slot=row_index.to_string()
            style=format!(
                "block-size: {height}px; transform: translateY({y}px);"
            )
        >
            {checkbox_cell}
            {cells}
        </tr>
    }
}

#[component]
pub fn RecordTable(
    columns: Vec<TableColumn>,
    total_rows: usize,
    #[prop(optional)] scroll_top: usize,
    #[prop(optional)] checkbox_column: bool,
) -> impl IntoView {
    let window = allocate_row_slots(scroll_top, total_rows);
    let header = columns.clone();
    let body_columns = columns;
    let rows: Vec<_> = (0..window.slot_count)
        .map(|slot| {
            let row_index = window.first_row_index + slot;
            view! {
                <TableRowSkeleton
                    row_index=row_index
                    columns=body_columns.clone()
                    checkbox=checkbox_column
                />
            }
        })
        .collect();
    let content_height = total_content_height(total_rows);
    view! {
        <table class=TABLE_CLASS>
            <thead class=TABLE_HEAD_CLASS>
                <tr class=TABLE_ROW_CLASS>
                    {checkbox_column.then(|| view! { <th class=TABLE_CHECKBOX_CELL_CLASS></th> })}
                    {header.into_iter().map(|c| view! { <TableHeaderCell column=c/> }).collect::<Vec<_>>()}
                </tr>
            </thead>
            <tbody
                class=TABLE_BODY_CLASS
                style=format!("block-size: {content_height}px; position: relative;")
                data-total-rows=total_rows.to_string()
            >
                {rows}
            </tbody>
        </table>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn virtualization_constants_match_upstream() {
        assert_eq!(ROW_SLOT_COUNT, 240);
        assert_eq!(RECORDS_PER_PAGE, 10);
        assert_eq!(OVERSCAN_PAGES, 7);
        assert_eq!(ROW_HEIGHT, 32);
        assert_eq!(COLUMN_MIN_WIDTH, 104);
        assert_eq!(CHECKBOX_COLUMN_WIDTH, 32);
        assert_eq!(TABLE_HORIZONTAL_CELL_MARGIN, 8);
        assert_eq!(TABLE_HORIZONTAL_CELL_PADDING, 8);
    }

    #[test]
    fn slot_allocation_is_240_slots_with_overscan() {
        assert_eq!(overscan_rows(), 70);
        let window = allocate_row_slots(0, 10_000);
        assert_eq!(window.first_row_index, 0);
        assert_eq!(window.slot_count, 240);
        let window = allocate_row_slots(3200, 10_000);
        assert_eq!(window.slot_count, 240);
        assert!(
            window.first_row_index <= 100 - overscan_rows(),
            "top clamped"
        );
    }

    #[test]
    fn slot_allocation_never_exceeds_pool_or_rows() {
        let window = allocate_row_slots(0, 50);
        assert_eq!(window.slot_count, 50);
        let window = allocate_row_slots(9_900 * ROW_HEIGHT, 10_000);
        assert!(window.slot_count <= ROW_SLOT_COUNT);
        assert!(window.first_row_index + window.slot_count <= 10_000);
    }

    #[test]
    fn row_offset_is_pure_offset_math_at_32px() {
        assert_eq!(row_offset_y(0), 0);
        assert_eq!(row_offset_y(10), 320);
        assert_eq!(total_content_height(10_000), 320_000);
    }

    #[test]
    fn row_skeleton_is_32px_and_slot_indexed() {
        let cols = vec![
            TableColumn::new("name", "Name"),
            TableColumn::new("email", "Email"),
        ];
        let out = (view! {
            <TableRowSkeleton row_index=7 columns=cols.clone() checkbox=true/>
        })
        .into_view()
        .to_html();
        assert!(out.contains(TABLE_ROW_CLASS), "{out}");
        assert!(out.contains("data-row-slot=\"7\""), "{out}");
        assert!(out.contains("block-size: 32px"), "{out}");
        assert!(out.contains("transform: translateY(224px)"), "{out}");
        assert!(out.matches("class=\"t-table-cell\"").count() == 2, "{out}");
        assert!(out.contains("min-inline-size: 104px"), "{out}");
        assert!(out.contains(TABLE_CHECKBOX_CELL_CLASS), "{out}");
        assert!(out.contains("inline-size: 32px"), "{out}");
        assert!(out.contains("type=\"checkbox\""), "{out}");
    }

    #[test]
    fn record_table_renders_the_slot_window_not_every_row() {
        let cols = vec![TableColumn::new("c", "Column")];
        let out = (view! { <RecordTable columns=cols.clone() total_rows=10000/> })
            .into_view()
            .to_html();
        let rendered_rows = out.matches("class=\"t-table-row\"").count();
        assert!(
            rendered_rows <= ROW_SLOT_COUNT + 1,
            "rendered {rendered_rows} rows, must stay within the {ROW_SLOT_COUNT} pool"
        );
        assert!(
            rendered_rows > 200,
            "should fill most slots: {rendered_rows}"
        );
        assert!(out.contains("data-total-rows=\"10000\""), "{out}");
        assert!(out.contains(TABLE_HEAD_CLASS), "{out}");
        assert!(out.contains(TABLE_BODY_CLASS), "{out}");
        assert!(out.contains(">Column<"), "{out}");
    }

    #[test]
    fn cell_margin_and_padding_use_the_table_tokens() {
        let cols = vec![TableColumn::new("a", "A")];
        let out = (view! {
            <RecordTable columns=cols total_rows=5 checkbox_column=true/>
        })
        .into_view()
        .to_html();
        assert!(out.contains("padding-inline: 8px"), "cell padding token");
        assert!(out.contains("margin-inline: 8px"), "header margin token");
    }
}
