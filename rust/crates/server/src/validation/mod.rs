pub mod composites;
pub mod date;
pub mod error;
pub mod inspect;
pub mod net;
pub mod record;
pub mod scalars;
pub mod values;

pub use composites::validate_actor_field;
pub use composites::validate_address_field;
pub use composites::validate_currency_field;
pub use composites::validate_emails_field;
pub use composites::validate_files_field;
pub use composites::validate_full_name_field;
pub use composites::validate_links_field;
pub use composites::validate_phones_field;
pub use composites::validate_rich_text_field;
pub use error::ValidationError;
pub use error::ValidationResult;
pub use record::validate_record_data;
pub use record::validate_records_data;
pub use scalars::validate_array_field;
pub use scalars::validate_boolean_field;
pub use scalars::validate_date_field;
pub use scalars::validate_date_time_field;
pub use scalars::validate_multi_select_field;
pub use scalars::validate_number_field;
pub use scalars::validate_numeric_field;
pub use scalars::validate_overridden_position_field;
pub use scalars::validate_rating_and_select_field;
pub use scalars::validate_raw_json_field;
pub use scalars::validate_text_field;
pub use scalars::validate_uuid_field;

use serde_json::Map;
use serde_json::Value;

use crate::query::runner::base::ObjectModel;
use crate::query::runner::RunnerError;

fn runner_error(error: ValidationError) -> RunnerError {
    RunnerError::from(error)
}

pub fn validate_for_runner(
    model: &ObjectModel,
    records: &[Map<String, Value>],
) -> Result<(), RunnerError> {
    validate_records_data(model, records).map_err(runner_error)
}

pub fn validate_record_for_runner(
    model: &ObjectModel,
    record: &Map<String, Value>,
) -> Result<(), RunnerError> {
    validate_record_data(model, record).map_err(runner_error)
}

pub fn validate_data_for_runner(model: &ObjectModel, data: &Value) -> Result<(), RunnerError> {
    match data {
        Value::Object(record) => validate_record_for_runner(model, record),
        Value::Array(items) => {
            for item in items.iter().filter_map(Value::as_object) {
                validate_record_for_runner(model, item)?;
            }

            Ok(())
        }
        _ => Ok(()),
    }
}
