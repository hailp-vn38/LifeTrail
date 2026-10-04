use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
    request_id: String,
}

pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
    request_id: String,
}

impl ApiError {
    pub fn invalid_request(request_id: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: "Request path is invalid.",
            request_id,
        }
    }

    pub fn invalid_token(request_id: String) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "invalid_token",
            message: "Device token is invalid.",
            request_id,
        }
    }

    pub fn not_found(request_id: String) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: "Resource was not found.",
            request_id,
        }
    }

    pub fn method_not_allowed(request_id: String) -> Self {
        Self {
            status: StatusCode::METHOD_NOT_ALLOWED,
            code: "method_not_allowed",
            message: "HTTP method is not allowed for this resource.",
            request_id,
        }
    }

    pub fn internal(request_id: String) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "The server could not complete the request.",
            request_id,
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
                },
            }),
        )
            .into_response()
    }
}
