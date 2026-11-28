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
pub use error::INVALID_ARGS_DATA;
pub use error::STANDARD_ERROR_MESSAGE;
pub use error::ValidationError;
pub use error::ValidationResult;
pub use record::validate_record_data;
pub use record::validate_records_data;

use serde_json::Map;
use serde_json::Value;

use crate::query::runner::base::ObjectModel;
use crate::query::runner::RunnerError;

pub fn validate_for_runner(
    model: &ObjectModel,
    records: &mut [Map<String, Value>],
) -> Result<(), RunnerError> {
    validate_records_data(model, records).map_err(RunnerError::from)
}
