use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RotationScheduled {
    pub secret_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialization_round_trip() {
        let event = RotationScheduled {
            secret_id: Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap(),
        };

        let json = serde_json::to_vec(&event).unwrap();
        let deserialized: RotationScheduled = serde_json::from_slice(&json).unwrap();

        assert_eq!(event, deserialized);
    }

    #[test]
    fn serializes_to_expected_json_format() {
        let event = RotationScheduled {
            secret_id: Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap(),
        };

        let json_str = serde_json::to_string(&event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(
            parsed["secret_id"], "08b81283-979b-4ce6-94c0-81311195201d",
            "secret_id should serialize as UUID string"
        );
    }

    #[test]
    fn deserializes_from_json_string() {
        let json = r#"{"secret_id":"08b81283-979b-4ce6-94c0-81311195201d"}"#;
        let event: RotationScheduled = serde_json::from_str(json).unwrap();

        assert_eq!(
            event.secret_id,
            Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap()
        );
    }

    #[test]
    fn rejects_missing_secret_id() {
        let json = r#"{}"#;
        let result = serde_json::from_str::<RotationScheduled>(json);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_uuid() {
        let json = r#"{"secret_id":"not-a-uuid"}"#;
        let result = serde_json::from_str::<RotationScheduled>(json);
        assert!(result.is_err());
    }
}
