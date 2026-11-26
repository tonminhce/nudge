use uuid::Uuid;

pub fn uuid_to_base36(id: &Uuid) -> String {
    let hex = id.simple().to_string();
    let value = u128::from_str_radix(&hex, 16).expect("a uuid is 32 hex digits");
    to_base36(value)
}

pub fn workspace_schema_name(workspace_id: &Uuid) -> String {
    format!("workspace_{}", uuid_to_base36(workspace_id))
}

fn to_base36(value: u128) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

    if value == 0 {
        return "0".to_string();
    }

    let mut digits = Vec::new();
    let mut remaining = value;

    while remaining > 0 {
        digits.push(DIGITS[(remaining % 36) as usize]);
        remaining /= 36;
    }

    digits.reverse();
    String::from_utf8(digits).expect("base36 digits are ascii")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base36_matches_the_javascript_implementation() {
        assert_eq!(uuid_to_base36(&Uuid::from_u128(42)), "16");
        assert_eq!(uuid_to_base36(&Uuid::from_u128(0)), "0");
        assert_eq!(uuid_to_base36(&Uuid::from_u128(35)), "z");
        assert_eq!(uuid_to_base36(&Uuid::from_u128(36)), "10");
    }

    #[test]
    fn schema_name_is_prefixed_with_the_base36_uuid() {
        let id = Uuid::from_u128(1_234_567_890);
        assert_eq!(
            workspace_schema_name(&id),
            format!("workspace_{}", uuid_to_base36(&id))
        );
        assert!(workspace_schema_name(&id).starts_with("workspace_"));
    }
}
