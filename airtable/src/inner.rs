use async_trait::async_trait;
use reqwest::{header, Method, Request, Response, Url};
use reqwest_middleware::RequestBuilder;
use std::sync::Arc;

use crate::error::AirtableError;

pub type Inner = Arc<dyn ApiClient>;

#[derive(Clone)]
pub struct InnerClient {
    key: String,
    base_id: String,
    enterprise_account_id: String,

    client: reqwest_middleware::ClientWithMiddleware,
}

impl InnerClient {
    pub fn new(
        key: String,
        base_id: String,
        enterprise_account_id: String,
        client: reqwest_middleware::ClientWithMiddleware,
    ) -> Self {
        Self {
            key,
            base_id,
            enterprise_account_id,
            client,
        }
    }
}

#[async_trait]
pub trait ApiClient {
    fn key(&self) -> &str;
    fn base_id(&self) -> &str;
    fn enterprise_account_id(&self) -> &str;
    fn client(&self) -> &reqwest_middleware::ClientWithMiddleware;

    fn request(
        &self,
        method: Method,
        url: Url,
        query: Option<Vec<(&str, String)>>,
    ) -> Result<RequestBuilder, AirtableError>;
    async fn execute(&self, request: Request) -> Result<Response, AirtableError>;
}

#[async_trait]
impl ApiClient for InnerClient {
    fn key(&self) -> &str {
        &self.key
    }

    fn base_id(&self) -> &str {
        &self.base_id
    }

    fn enterprise_account_id(&self) -> &str {
        &self.enterprise_account_id
    }

    fn client(&self) -> &reqwest_middleware::ClientWithMiddleware {
        &self.client
    }

    fn request(
        &self,
        method: Method,
        url: Url,
        query: Option<Vec<(&str, String)>>,
    ) -> Result<RequestBuilder, AirtableError> {
        let bt = format!("Bearer {}", self.key());
        let bearer = header::HeaderValue::from_str(&bt).map_err(|_| AirtableError::FailedToConstructRequest)?;

        // Set the default headers.
        let mut headers = header::HeaderMap::new();
        headers.append(header::AUTHORIZATION, bearer);
        headers.append(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let mut rb = self.client.request(method.clone(), url).headers(headers);

        match query {
            None => (),
            Some(val) => {
                rb = rb.query(&val);
            }
        }

        Ok(rb)
    }

    async fn execute(&self, request: Request) -> Result<Response, AirtableError> {
        Ok(self.client.execute(request).await?)
    }
}

// fn request<B>(&self, method: Method, path: String, body: B, query: Option<Vec<(&str, String)>>) -> Result<Request>
// where
//     B: Serialize,
// {
//     let base = Url::parse(ENDPOINT)?;
//     let url = base.join(&(self.inner.base_id.to_string() + "/" + &path))?;

//     let bt = format!("Bearer {}", self.get_key());
//     let bearer = header::HeaderValue::from_str(&bt)?;

//     // Set the default headers.
//     let mut headers = header::HeaderMap::new();
//     headers.append(header::AUTHORIZATION, bearer);
//     headers.append(
//         header::CONTENT_TYPE,
//         header::HeaderValue::from_static("application/json"),
//     );

//     let mut rb = self.inner.client.request(method.clone(), url).headers(headers);

//     match query {
//         None => (),
//         Some(val) => {
//             rb = rb.query(&val);
//         }
//     }

//     // Add the body, this is to ensure our GET and DELETE calls succeed.
//     if method != Method::GET && method != Method::DELETE {
//         rb = rb.json(&body);
//     }

//     // Build the request.
//     Ok(rb.build()?)
// }