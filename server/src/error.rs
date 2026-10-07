use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
    request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    request_id: String,
    details: Option<Value>,
}

impl ApiError {
    pub fn invalid_request(request_id: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: "Request path is invalid.",
            request_id,
            details: None,
        }
    }

    pub fn invalid_token(request_id: String) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "invalid_token",
            message: "Device token is invalid.",
            request_id,
            details: None,
        }
    }

    pub fn not_found(request_id: String) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: "Resource was not found.",
            request_id,
            details: None,
        }
    }

    pub fn method_not_allowed(request_id: String) -> Self {
        Self {
            status: StatusCode::METHOD_NOT_ALLOWED,
            code: "method_not_allowed",
            message: "HTTP method is not allowed for this resource.",
            request_id,
            details: None,
        }
    }

    pub fn gone(request_id: String) -> Self {
        Self {
            status: StatusCode::GONE,
            code: "manifest_expired",
            message: "Pinned playback manifest is no longer available.",
            request_id,
            details: None,
        }
    }

    pub fn internal(request_id: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "The server could not complete the request.",
            request_id,
            details: None,
        }
    }

    pub fn invalid_batch(request_id: String, details: Option<Value>) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "invalid_batch",
            message: "Batch contains an invalid GPS Record.",
            request_id,
            details,
        }
    }

    pub fn batch_conflict(request_id: String) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "batch_conflict",
            message: "Batch ID was already committed with different content.",
            request_id,
            details: None,
        }
    }

    pub fn body_too_large(request_id: String) -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "body_too_large",
            message: "Batch body exceeds the maximum size.",
            request_id,
            details: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    code: self.code,
                    message: self.message,
                    request_id: self.request_id,
                    details: self.details,
                },
            }),
        )
            .into_response()
    }
}
