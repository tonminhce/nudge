#![allow(dead_code)]

use crate::metadata::flat::FieldMetadataType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderByDirection {
    AscNullsFirst,
    AscNullsLast,
    DescNullsFirst,
    DescNullsLast,
}

impl OrderByDirection {
    pub fn parse(value: &str) -> Option<Self> {
        let direction = match value {
            "AscNullsFirst" => OrderByDirection::AscNullsFirst,
            "AscNullsLast" => OrderByDirection::AscNullsLast,
            "DescNullsFirst" => OrderByDirection::DescNullsFirst,
            "DescNullsLast" => OrderByDirection::DescNullsLast,
            _ => return None,
        };

        Some(direction)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            OrderByDirection::AscNullsFirst => "AscNullsFirst",
            OrderByDirection::AscNullsLast => "AscNullsLast",
            OrderByDirection::DescNullsFirst => "DescNullsFirst",
            OrderByDirection::DescNullsLast => "DescNullsLast",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullsPosition {
    First,
    Last,
}

impl NullsPosition {
    pub fn as_str(self) -> &'static str {
        match self {
            NullsPosition::First => "NULLS FIRST",
            NullsPosition::Last => "NULLS LAST",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderByClause {
    pub order: SortDirection,
    pub nulls: NullsPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveScanOrder {
    pub is_ascending: bool,
    pub are_nulls_scanned_last: bool,
}

pub fn is_ascending_order(direction: OrderByDirection) -> bool {
    matches!(
        direction,
        OrderByDirection::AscNullsFirst | OrderByDirection::AscNullsLast
    )
}

pub fn get_effective_scan_order(
    direction: OrderByDirection,
    is_forward_pagination: bool,
) -> EffectiveScanOrder {
    let are_nulls_presented_last = matches!(
        direction,
        OrderByDirection::AscNullsLast | OrderByDirection::DescNullsLast
    );

    EffectiveScanOrder {
        is_ascending: is_ascending_order(direction) == is_forward_pagination,
        are_nulls_scanned_last: if is_forward_pagination {
            are_nulls_presented_last
        } else {
            !are_nulls_presented_last
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderError {
    InvalidDirection(String),
}

impl OrderError {
    pub fn code(&self) -> &'static str {
        match self {
            OrderError::InvalidDirection(_) => "INVALID_DIRECTION",
        }
    }

    pub fn message(&self) -> String {
        match self {
            OrderError::InvalidDirection(direction) => format!("Invalid direction: {direction}"),
        }
    }
}

pub fn convert_order_by_to_find_options_order(
    direction: &str,
    is_forward_pagination: bool,
) -> Result<OrderByClause, OrderError> {
    let direction = OrderByDirection::parse(direction)
        .ok_or_else(|| OrderError::InvalidDirection(direction.to_string()))?;
    let scan = get_effective_scan_order(direction, is_forward_pagination);

    Ok(OrderByClause {
        order: if scan.is_ascending {
            SortDirection::Asc
        } else {
            SortDirection::Desc
        },
        nulls: if scan.are_nulls_scanned_last {
            NullsPosition::Last
        } else {
            NullsPosition::First
        },
    })
}

pub fn should_use_case_insensitive_order(field_type: FieldMetadataType) -> bool {
    matches!(
        field_type,
        FieldMetadataType::Text | FieldMetadataType::Select | FieldMetadataType::MultiSelect
    )
}

pub fn should_cast_to_text(field_type: FieldMetadataType) -> bool {
    matches!(
        field_type,
        FieldMetadataType::Select | FieldMetadataType::MultiSelect
    )
}

pub fn build_order_by_column_expression(prefix: &str, column_name: &str) -> String {
    format!("{prefix}.{column_name}")
}

pub const DEFAULT_ID_ORDER_BY_TIEBREAKER: OrderByDirection = OrderByDirection::AscNullsLast;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_scan_order_matches_the_captured_matrix() {
        #[allow(clippy::type_complexity)]
        let cases: Vec<(&str, bool, (bool, bool))> = vec![
            ("AscNullsFirst", true, (true, false)),
            ("AscNullsFirst", false, (false, true)),
            ("AscNullsLast", true, (true, true)),
            ("AscNullsLast", false, (false, false)),
            ("DescNullsFirst", true, (false, false)),
            ("DescNullsFirst", false, (true, true)),
            ("DescNullsLast", true, (false, true)),
            ("DescNullsLast", false, (true, false)),
        ];

        for (direction, is_forward, (is_ascending, nulls_last)) in cases {
            let parsed = OrderByDirection::parse(direction).expect("valid direction");
            let scan = get_effective_scan_order(parsed, is_forward);
            assert_eq!(
                (scan.is_ascending, scan.are_nulls_scanned_last),
                (is_ascending, nulls_last),
                "{direction} forward={is_forward}"
            );
        }
    }

    #[test]
    fn direction_to_find_options_order_matches_the_captured_matrix() {
        #[allow(clippy::type_complexity)]
        let cases: Vec<(&str, bool, Result<(&str, &str), (&str, &str)>)> = vec![
            ("AscNullsFirst", true, Ok(("ASC", "NULLS FIRST"))),
            ("AscNullsFirst", false, Ok(("DESC", "NULLS LAST"))),
            ("AscNullsLast", true, Ok(("ASC", "NULLS LAST"))),
            ("AscNullsLast", false, Ok(("DESC", "NULLS FIRST"))),
            ("DescNullsFirst", true, Ok(("DESC", "NULLS FIRST"))),
            ("DescNullsFirst", false, Ok(("ASC", "NULLS LAST"))),
            ("DescNullsLast", true, Ok(("DESC", "NULLS LAST"))),
            ("DescNullsLast", false, Ok(("ASC", "NULLS FIRST"))),
            (
                "Nope",
                true,
                Err(("INVALID_DIRECTION", "Invalid direction: Nope")),
            ),
        ];

        for (direction, is_forward, expected) in cases {
            let result = convert_order_by_to_find_options_order(direction, is_forward);

            match expected {
                Ok((order, nulls)) => {
                    let clause = result.expect("valid direction must convert");
                    assert_eq!(
                        (clause.order.as_str(), clause.nulls.as_str()),
                        (order, nulls),
                        "{direction} forward={is_forward}"
                    );
                }
                Err((code, message)) => {
                    let error = result.expect_err("invalid direction must fail");
                    assert_eq!(error.code(), code, "{direction}");
                    assert_eq!(error.message(), message, "{direction}");
                }
            }
        }
    }

    #[test]
    fn invalid_direction_carries_the_invalid_direction_code() {
        let error = convert_order_by_to_find_options_order("Sideways", true).unwrap_err();
        assert_eq!(error.code(), "INVALID_DIRECTION");
        assert_eq!(error.message(), "Invalid direction: Sideways");
    }

    #[test]
    fn is_ascending_order_covers_both_asc_variants() {
        assert!(is_ascending_order(OrderByDirection::AscNullsFirst));
        assert!(is_ascending_order(OrderByDirection::AscNullsLast));
        assert!(!is_ascending_order(OrderByDirection::DescNullsFirst));
        assert!(!is_ascending_order(OrderByDirection::DescNullsLast));
    }

    #[test]
    fn backward_pagination_flips_both_axes_of_the_scan() {
        for direction in [
            OrderByDirection::AscNullsFirst,
            OrderByDirection::AscNullsLast,
            OrderByDirection::DescNullsFirst,
            OrderByDirection::DescNullsLast,
        ] {
            let forward = get_effective_scan_order(direction, true);
            let backward = get_effective_scan_order(direction, false);
            assert_ne!(forward.is_ascending, backward.is_ascending, "{direction:?}");
            assert_ne!(
                forward.are_nulls_scanned_last, backward.are_nulls_scanned_last,
                "{direction:?}"
            );
        }
    }

    #[test]
    fn textish_columns_get_case_insensitive_ordering_and_text_casts() {
        for field_type in [
            FieldMetadataType::Text,
            FieldMetadataType::Select,
            FieldMetadataType::MultiSelect,
        ] {
            assert!(
                should_use_case_insensitive_order(field_type),
                "{field_type:?}"
            );
        }
        assert!(!should_use_case_insensitive_order(
            FieldMetadataType::Number
        ));
        assert!(!should_use_case_insensitive_order(FieldMetadataType::Uuid));
        assert!(should_cast_to_text(FieldMetadataType::Select));
        assert!(should_cast_to_text(FieldMetadataType::MultiSelect));
        assert!(!should_cast_to_text(FieldMetadataType::Text));
    }

    #[test]
    fn order_by_column_expression_keeps_the_unquoted_prefix_dot_column_shape() {
        assert_eq!(
            build_order_by_column_expression("company", "name"),
            "company.name"
        );
    }

    #[test]
    fn the_default_id_tiebreaker_is_asc_nulls_last() {
        assert_eq!(
            DEFAULT_ID_ORDER_BY_TIEBREAKER,
            OrderByDirection::AscNullsLast
        );
    }
}
