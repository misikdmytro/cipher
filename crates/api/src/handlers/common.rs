use serde::Serialize;
use utoipa::ToSchema;
use validator::ValidationErrors;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    #[schema(example = "Invalid request: path cannot be empty")]
    pub message: String,
}

impl From<ValidationErrors> for ErrorResponse {
    fn from(error: ValidationErrors) -> Self {
        let message = error
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                let messages: Vec<String> = errors
                    .iter()
                    .map(|error| {
                        if let Some(message) = &error.message {
                            message.to_string()
                        } else {
                            format!("Invalid value for '{}'", field)
                        }
                    })
                    .collect();
                messages.join(", ")
            })
            .collect::<Vec<String>>()
            .join("; ");

        ErrorResponse { message }
    }
}
