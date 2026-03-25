use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RotationScheduled {
    pub secret_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RotationStarted {
    pub secret_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RotationDone {
    pub secret_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RotationFailed {
    pub secret_id: Uuid,
    pub error: String,
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

    // RotationStarted tests

    #[test]
    fn started_serialization_round_trip() {
        let event = RotationStarted {
            secret_id: Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap(),
        };

        let json = serde_json::to_vec(&event).unwrap();
        let deserialized: RotationStarted = serde_json::from_slice(&json).unwrap();

        assert_eq!(event, deserialized);
    }

    #[test]
    fn started_serializes_to_expected_json_format() {
        let event = RotationStarted {
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
    fn started_deserializes_from_json_string() {
        let json = r#"{"secret_id":"08b81283-979b-4ce6-94c0-81311195201d"}"#;
        let event: RotationStarted = serde_json::from_str(json).unwrap();

        assert_eq!(
            event.secret_id,
            Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap()
        );
    }

    #[test]
    fn started_rejects_missing_secret_id() {
        let json = r#"{}"#;
        let result = serde_json::from_str::<RotationStarted>(json);
        assert!(result.is_err());
    }

    #[test]
    fn started_rejects_invalid_uuid() {
        let json = r#"{"secret_id":"not-a-uuid"}"#;
        let result = serde_json::from_str::<RotationStarted>(json);
        assert!(result.is_err());
    }

    // RotationDone tests

    #[test]
    fn done_serialization_round_trip() {
        let event = RotationDone {
            secret_id: Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap(),
        };

        let json = serde_json::to_vec(&event).unwrap();
        let deserialized: RotationDone = serde_json::from_slice(&json).unwrap();

        assert_eq!(event, deserialized);
    }

    #[test]
    fn done_serializes_to_expected_json_format() {
        let event = RotationDone {
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
    fn done_deserializes_from_json_string() {
        let json = r#"{"secret_id":"08b81283-979b-4ce6-94c0-81311195201d"}"#;
        let event: RotationDone = serde_json::from_str(json).unwrap();

        assert_eq!(
            event.secret_id,
            Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap()
        );
    }

    #[test]
    fn done_rejects_missing_secret_id() {
        let json = r#"{}"#;
        let result = serde_json::from_str::<RotationDone>(json);
        assert!(result.is_err());
    }

    #[test]
    fn done_rejects_invalid_uuid() {
        let json = r#"{"secret_id":"not-a-uuid"}"#;
        let result = serde_json::from_str::<RotationDone>(json);
        assert!(result.is_err());
    }

    // RotationFailed tests

    #[test]
    fn failed_serialization_round_trip() {
        let event = RotationFailed {
            secret_id: Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap(),
            error: "access denied".to_string(),
        };

        let json = serde_json::to_vec(&event).unwrap();
        let deserialized: RotationFailed = serde_json::from_slice(&json).unwrap();

        assert_eq!(event, deserialized);
    }

    #[test]
    fn failed_serializes_to_expected_json_format() {
        let event = RotationFailed {
            secret_id: Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap(),
            error: "access denied".to_string(),
        };

        let json_str = serde_json::to_string(&event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(
            parsed["secret_id"], "08b81283-979b-4ce6-94c0-81311195201d",
            "secret_id should serialize as UUID string"
        );
        assert_eq!(parsed["error"], "access denied");
    }

    #[test]
    fn failed_deserializes_from_json_string() {
        let json =
            r#"{"secret_id":"08b81283-979b-4ce6-94c0-81311195201d","error":"access denied"}"#;
        let event: RotationFailed = serde_json::from_str(json).unwrap();

        assert_eq!(
            event.secret_id,
            Uuid::parse_str("08b81283-979b-4ce6-94c0-81311195201d").unwrap()
        );
        assert_eq!(event.error, "access denied");
    }

    #[test]
    fn failed_rejects_missing_secret_id() {
        let json = r#"{"error":"something"}"#;
        let result = serde_json::from_str::<RotationFailed>(json);
        assert!(result.is_err());
    }

    #[test]
    fn failed_rejects_missing_error() {
        let json = r#"{"secret_id":"08b81283-979b-4ce6-94c0-81311195201d"}"#;
        let result = serde_json::from_str::<RotationFailed>(json);
        assert!(result.is_err());
    }

    #[test]
    fn failed_rejects_invalid_uuid() {
        let json = r#"{"secret_id":"not-a-uuid","error":"something"}"#;
        let result = serde_json::from_str::<RotationFailed>(json);
        assert!(result.is_err());
    }
}
