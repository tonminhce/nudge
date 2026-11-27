use std::collections::BTreeMap;
use std::sync::Arc;

use front_ui::icons::IconCalendar;
use front_ui::icons::IconChevronLeft;
use front_ui::icons::IconChevronRight;
use front_ui::icons::IconLayoutKanban;
use front_ui::icons::IconLayoutList;
use front_ui::icons::IconTable;
use js_sys::Date;
use leptos::prelude::*;
use leptos_router::components::A;
use serde_json::Value;
use wasm_bindgen::JsCast;

use crate::record_index::company_name;
use crate::record_index::created_at;
use crate::record_index::display_name;
use crate::record_index::primary_email;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordViewType {
    Table,
    Kanban,
    Calendar,
    List,
}

impl RecordViewType {
    pub fn parse(value: &str) -> Self {
        match value {
            "kanban" => Self::Kanban,
            "calendar" => Self::Calendar,
            "list" => Self::List,
            _ => Self::Table,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Kanban => "kanban",
            Self::Calendar => "calendar",
            Self::List => "list",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Table => "Table",
            Self::Kanban => "Kanban",
            Self::Calendar => "Calendar",
            Self::List => "List",
        }
    }
}

pub fn view_type_icon(view_type: RecordViewType) -> AnyView {
    match view_type {
        RecordViewType::Table => view! { <IconTable size=16 /> }.into_any(),
        RecordViewType::Kanban => view! { <IconLayoutKanban size=16 /> }.into_any(),
        RecordViewType::Calendar => view! { <IconCalendar size=16 /> }.into_any(),
        RecordViewType::List => view! { <IconLayoutList size=16 /> }.into_any(),
    }
}

pub fn alternate_view(view_type: RecordViewType, records: Vec<Value>, singular: String) -> AnyView {
    match view_type {
        RecordViewType::List => {
            view! { <RecordListView records=records singular=singular /> }.into_any()
        }
        RecordViewType::Kanban => {
            view! { <RecordKanbanView records=records singular=singular /> }.into_any()
        }
        RecordViewType::Calendar => {
            view! { <RecordCalendarView records=records singular=singular /> }.into_any()
        }
        RecordViewType::Table => view! { <span /> }.into_any(),
    }
}

#[component]
pub fn RecordListView(records: Vec<Value>, singular: String) -> impl IntoView {
    let singular = RwSignal::new(singular);
    view! {
        <div class="record-list" role="list" aria-label="Records list">
            <For each=move || records.clone() key=|record| record["id"].as_str().unwrap_or_default().to_string() let(record)>
                {
                    let record_id = record["id"].as_str().unwrap_or_default().to_string();
                    view! {
                        <A href=move || format!("/object/{}/{record_id}", singular.get()) attr:class="record-list-row" attr:role="listitem">
                            <span class="record-list-name">{display_name(&record)}</span>
                            <span class="record-list-fields">
                                <span>{company_name(&record)}</span>
                                <span>{primary_email(&record)}</span>
                                <span>{record["jobTitle"].as_str().unwrap_or_default().to_string()}</span>
                            </span>
                            <span class="record-list-date">{created_at(&record)}</span>
                        </A>
                    }
                }
            </For>
        </div>
    }
}

fn group_by_company(records: &[Value]) -> BTreeMap<String, Vec<Value>> {
    let mut groups = BTreeMap::new();
    for record in records {
        let company = company_name(record);
        let company = if company.is_empty() {
            String::from("No company")
        } else {
            company
        };
        groups
            .entry(company)
            .or_insert_with(Vec::new)
            .push(record.clone());
    }
    groups
}

#[component]
pub fn RecordKanbanView(records: Vec<Value>, singular: String) -> impl IntoView {
    let columns = group_by_company(&records).into_iter().collect::<Vec<_>>();
    let singular = RwSignal::new(singular);
    view! {
        <div class="record-kanban" role="region" aria-label="Kanban board">
            <For each=move || columns.clone() key=|(name, _)| name.clone() let((name, group_records))>
                <section class="record-kanban-column">
                    <header class="record-kanban-header">
                        <span>{name}</span>
                        <span class="record-kanban-count">{group_records.len()}</span>
                    </header>
                    <div class="record-kanban-cards">
                        <For each=move || group_records.clone() key=|record| record["id"].as_str().unwrap_or_default().to_string() let(record)>
                            {
                                let record_id = record["id"].as_str().unwrap_or_default().to_string();
                                view! {
                                    <A href=move || format!("/object/{}/{record_id}", singular.get()) attr:class="record-kanban-card">
                                        <span class="record-kanban-card-title">{display_name(&record)}</span>
                                        <span class="record-kanban-card-email">{primary_email(&record)}</span>
                                        <span class="record-kanban-card-company">{company_name(&record)}</span>
                                    </A>
                                }
                            }
                        </For>
                    </div>
                </section>
            </For>
        </div>
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CalendarLayout {
    Day,
    Week,
    Month,
}

impl CalendarLayout {
    fn parse(value: &str) -> Self {
        match value {
            "day" => Self::Day,
            "week" => Self::Week,
            _ => Self::Month,
        }
    }
}

fn parse_date(value: &str) -> Option<(i32, u32, u32)> {
    if value.len() < 10 {
        return None;
    }
    let year = value.get(0..4)?.parse().ok()?;
    let month = value.get(5..7)?.parse().ok()?;
    let day = value.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

fn current_date() -> (i32, u32, u32) {
    let date = Date::new_0();
    (
        date.get_full_year() as i32,
        date.get_month() + 1,
        date.get_date(),
    )
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let year = year - i32::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let adjusted_month = month as i32 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day as i32 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era - 719468
}

fn civil_from_days(mut days: i32) -> (i32, u32, u32) {
    days += 719468;
    let era = days.div_euclid(146097);
    let day_of_era = days - era * 146097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i32::from(month <= 2);
    (year, month as u32, day as u32)
}

fn shift_days(date: (i32, u32, u32), amount: i32) -> (i32, u32, u32) {
    civil_from_days(days_from_civil(date.0, date.1, date.2) + amount)
}

fn shift_months(date: (i32, u32, u32), amount: i32) -> (i32, u32, u32) {
    let month_index = date.0 * 12 + date.1 as i32 - 1 + amount;
    let year = month_index.div_euclid(12);
    let month = month_index.rem_euclid(12) as u32 + 1;
    (year, month, date.2.min(days_in_month(year, month)))
}

fn month_weeks(year: i32, month: u32) -> Vec<Vec<Option<(i32, u32, u32)>>> {
    let offset = (days_from_civil(year, month, 1) + 3).rem_euclid(7) as usize;
    let total_days = days_in_month(year, month) as usize;
    let cell_count = (offset + total_days).next_multiple_of(7);
    (0..cell_count)
        .step_by(7)
        .map(|start| {
            (start..start + 7)
                .map(|index| {
                    index
                        .checked_sub(offset)
                        .filter(|day| *day < total_days)
                        .map(|day| (year, month, day as u32 + 1))
                })
                .collect()
        })
        .collect()
}

fn week_days(date: (i32, u32, u32)) -> Vec<Option<(i32, u32, u32)>> {
    let weekday = (days_from_civil(date.0, date.1, date.2) + 3).rem_euclid(7);
    let monday = shift_days(date, -weekday);
    (0..7)
        .map(|offset| Some(shift_days(monday, offset)))
        .collect()
}

fn format_date(date: (i32, u32, u32)) -> String {
    format!("{:04}-{:02}-{:02}", date.0, date.1, date.2)
}

fn group_by_day(records: &[Value]) -> BTreeMap<String, Vec<Value>> {
    let mut groups = BTreeMap::new();
    for record in records {
        let Some(date) = record["createdAt"].as_str().and_then(parse_date) else {
            continue;
        };
        groups
            .entry(format_date(date))
            .or_insert_with(Vec::new)
            .push(record.clone());
    }
    groups
}

fn calendar_weeks(
    layout: CalendarLayout,
    selected: (i32, u32, u32),
) -> Vec<Vec<Option<(i32, u32, u32)>>> {
    match layout {
        CalendarLayout::Month => month_weeks(selected.0, selected.1),
        CalendarLayout::Week => vec![week_days(selected)],
        CalendarLayout::Day => vec![vec![Some(selected)]],
    }
}

#[component]
pub fn RecordCalendarView(records: Vec<Value>, singular: String) -> impl IntoView {
    let initial = records
        .iter()
        .find_map(|record| record["createdAt"].as_str().and_then(parse_date))
        .unwrap_or_else(current_date);
    let selected = RwSignal::new(initial);
    let layout = RwSignal::new(CalendarLayout::Month);
    let records_by_day = Arc::new(group_by_day(&records));
    let month_label = move || {
        let date = selected.get();
        format!("{} {}", MONTHS[date.1 as usize - 1], date.0)
    };

    view! {
        <div class="record-calendar" role="region" aria-label="Record calendar">
            <header class="record-calendar-topbar">
                <div class="record-calendar-topbar-left">
                    <select class="record-sort" aria-label="Calendar layout" on:change=move |event| {
                        let value = event.target()
                            .and_then(|target| target.dyn_into::<web_sys::HtmlSelectElement>().ok())
                            .map(|select| select.value())
                            .unwrap_or_default();
                        layout.set(CalendarLayout::parse(&value));
                    }>
                        <option value="day">"Day"</option>
                        <option value="week">"Week"</option>
                        <option value="month" selected=true>"Month"</option>
                    </select>
                    <span class="record-calendar-month">{month_label}</span>
                </div>
                <div class="record-calendar-topbar-right">
                    <button class="app-icon-button" aria-label="Previous period" on:click=move |_| {
                        selected.update(|date| *date = match layout.get() {
                            CalendarLayout::Month => shift_months(*date, -1),
                            CalendarLayout::Week => shift_days(*date, -7),
                            CalendarLayout::Day => shift_days(*date, -1),
                        });
                    }><IconChevronLeft size=16 /></button>
                    <button class="t-button" data-variant="ghost" data-color="neutral" data-size="sm" on:click=move |_| selected.set(current_date())>"Today"</button>
                    <button class="app-icon-button" aria-label="Next period" on:click=move |_| {
                        selected.update(|date| *date = match layout.get() {
                            CalendarLayout::Month => shift_months(*date, 1),
                            CalendarLayout::Week => shift_days(*date, 7),
                            CalendarLayout::Day => shift_days(*date, 1),
                        });
                    }><IconChevronRight size=16 /></button>
                </div>
            </header>
            <div class="record-calendar-weekdays">
                <span>"Mon"</span><span>"Tue"</span><span>"Wed"</span><span>"Thu"</span><span>"Fri"</span><span>"Sat"</span><span>"Sun"</span>
            </div>
            {move || {
                let weeks = calendar_weeks(layout.get(), selected.get());
                view! {
                    <CalendarGrid weeks=weeks records_by_day=records_by_day.clone() selected=selected singular=singular.clone() />
                }
            }}
        </div>
    }
}

#[component]
fn CalendarGrid(
    weeks: Vec<Vec<Option<(i32, u32, u32)>>>,
    records_by_day: Arc<BTreeMap<String, Vec<Value>>>,
    selected: RwSignal<(i32, u32, u32)>,
    singular: String,
) -> impl IntoView {
    let records_by_day = RwSignal::new(records_by_day);
    let singular = RwSignal::new(singular);
    view! {
        <div class="record-calendar-grid">
            <For each=move || weeks.clone() key=|index| index.clone() let(week)>
                <div class="record-calendar-week">
                    <For each=move || week.clone() key=|index| *index let(date)>
                        {
                            let records_for_cell = records_by_day.get();
                            view! {
                                <div class="record-calendar-day" data-selected=move || if date.is_some_and(|date| selected.get() == date) { "true" } else { "false" }>
                                    {move || date.map(|date| {
                                        let day_records = records_for_cell.get(&format_date(date)).cloned().unwrap_or_default();
                                        view! {
                                            <span class="record-calendar-day-number">{date.2}</span>
                                            <div class="record-calendar-day-records">
                                                <For each=move || day_records.clone() key=|record| record["id"].as_str().unwrap_or_default().to_string() let(record)>
                                                    {
                                                        let record_id = record["id"].as_str().unwrap_or_default().to_string();
                                                        view! {
                                                            <A href=move || format!("/object/{}/{record_id}", singular.get()) attr:class="record-calendar-card">
                                                                {display_name(&record)}
                                                            </A>
                                                        }
                                                    }
                                                </For>
                                            </div>
                                        }
                                    })}
                                </div>
                            }
                        }
                    </For>
                </div>
            </For>
        </div>
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_type_parsing_matches_upstream_names() {
        assert_eq!(RecordViewType::parse("table"), RecordViewType::Table);
        assert_eq!(RecordViewType::parse("kanban"), RecordViewType::Kanban);
        assert_eq!(RecordViewType::parse("calendar"), RecordViewType::Calendar);
        assert_eq!(RecordViewType::parse("list"), RecordViewType::List);
        assert_eq!(RecordViewType::parse("other"), RecordViewType::Table);
    }

    #[test]
    fn month_grid_starts_on_monday_and_clamps_the_month() {
        let weeks = month_weeks(2026, 11);
        assert_eq!(weeks.len(), 6);
        assert_eq!(weeks[0][0], None);
        assert_eq!(weeks[0][6], Some((2026, 11, 1)));
        assert_eq!(
            weeks.iter().flatten().filter_map(|date| *date).next_back(),
            Some((2026, 11, 30))
        );
    }

    #[test]
    fn week_grid_starts_on_monday_across_month_boundaries() {
        assert_eq!(
            week_days((2026, 11, 27)),
            vec![
                Some((2026, 11, 23)),
                Some((2026, 11, 24)),
                Some((2026, 11, 25)),
                Some((2026, 11, 26)),
                Some((2026, 11, 27)),
                Some((2026, 11, 28)),
                Some((2026, 11, 29)),
            ]
        );
    }

    #[test]
    fn day_layout_keeps_only_the_selected_date() {
        assert_eq!(
            calendar_weeks(CalendarLayout::Day, (2026, 11, 27)),
            vec![vec![Some((2026, 11, 27))]]
        );
    }

    #[test]
    fn date_shifts_handle_leap_years_and_short_months() {
        assert_eq!(shift_months((2024, 1, 31), 1), (2024, 2, 29));
        assert_eq!(shift_months((2023, 1, 31), 1), (2023, 2, 28));
        assert_eq!(shift_days((2026, 12, 31), 1), (2027, 1, 1));
    }

    #[test]
    fn kanban_groups_use_company_and_keep_empty_relations() {
        let records = vec![
            serde_json::json!({"id": "1", "company": {"name": "Acme"}}),
            serde_json::json!({"id": "2", "company": {"name": "Acme"}}),
            serde_json::json!({"id": "3", "company": null}),
        ];
        let groups = group_by_company(&records);
        assert_eq!(groups["Acme"].len(), 2);
        assert_eq!(groups["No company"].len(), 1);
    }
}
