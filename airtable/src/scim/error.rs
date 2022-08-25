use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

use crate::error::ClientError;

#[derive(Debug, Clone, JsonSchema, Serialize)]
pub enum ScimClientError {
    Api(AirtableScimApiError),
    Client(ClientError),
}

impl fmt::Display for ScimClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Api(api_error) => write!(
                f,
                "Request failed with {} due to {}",
                api_error.status, api_error.detail
            ),
            Self::Client(inner) => write!(f, "Failed due to client error {}", inner),
        }
    }
}

impl Error for ScimClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Api(_) => None,
            Self::Client(inner) => Some(inner),
        }
    }
}

impl<T> From<T> for ScimClientError
where
    T: Into<ClientError>,
{
    fn from(err: T) -> Self {
        Self::Client(err.into())
    }
}

#[derive(Debug, Clone, PartialEq, JsonSchema, Deserialize, Serialize)]
pub struct AirtableScimApiError {
    pub schemas: Vec<String>,
    pub status: u16,
    pub detail: String,
}
