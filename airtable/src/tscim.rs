use chrono::{DateTime, Utc};
use reqwest::{Method, Response, StatusCode, Url};
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    error::{
        AirtableError,
        AirtableEnterpriseError,
    },
    inner::Inner
};

#[derive(Clone)]
pub struct AirtableScimClient {
    inner: Inner,
}

impl AirtableScimClient {
    pub(crate) fn new(inner: Inner) -> Self {
        Self { inner }
    }

    pub fn user(&self) -> AirtableScimUserClient {
        AirtableScimUserClient {
            inner: self.inner.clone(),
        }
    }

    pub fn group(&self) -> AirtableScimGroupClient {
        AirtableScimGroupClient {
            inner: self.inner.clone(),
        }
    }
}

pub struct AirtableScimUserClient {
    inner: Inner,
}

impl AirtableScimUserClient {
    fn base_endpoint() -> &'static str {
        "https://airtable.com/scim/v2/Users"
    }

    fn url(base: &str, path: Option<&str>) -> Result<Url, ScimError> {
        let url = Url::parse(base)?;

        if let Some(path) = path {
            Ok(url.join("/")?.join(path)?)
        } else {
            Ok(url)
        }
    }

    /// From: https://airtable.com/api/enterprise#scimUsersGet
    pub async fn list(&self) -> Result<ScimListResponse<ScimUser>, ScimError> {
        let req = self
            .inner
            .request(Method::GET, Self::url(Self::base_endpoint(), None)?, None)?
            .body("")
            .build()?;
        let resp = self.inner.execute(req).await?;

        to_client_response(resp).await
    }

    /// From: https://airtable.com/api/enterprise#scimUsersGetById
    pub async fn get<T: AsRef<str>>(&self, id: T) -> Result<Option<ScimUser>, ScimError> {
        let req = self
            .inner
            .request(Method::GET, Self::url(Self::base_endpoint(), Some(id.as_ref()))?, None)?
            .body("")
            .build()?;
        let resp = self.inner.execute(req).await?;

        to_client_response(resp).await
    }

    /// From: https://airtable.com/api/enterprise#scimUserCreate
    pub async fn create(&self, new_user: &ScimCreateUser) -> Result<ScimUser, ScimError> {
        let req = self
            .inner
            .request(Method::POST, Self::url(Self::base_endpoint(), None)?, None)?
            .body(serde_json::to_string(new_user)?)
            .build()?;
        let resp = self.inner.execute(req).await?;

        to_client_response(resp).await
    }

    /// From: https://airtable.com/api/enterprise#scimUserUpdate
    pub async fn update<T: AsRef<str>>(&self, id: T, user: &ScimUpdateUser) -> Result<ScimUser, ScimError> {
        let req = self
            .inner
            .request(Method::PUT, Self::url(Self::base_endpoint(), Some(id.as_ref()))?, None)?
            .body(serde_json::to_string(user)?)
            .build()?;
        let resp = self.inner.execute(req).await?;

        to_client_response(resp).await
    }

    // /// From: https://airtable.com/api/enterprise#scimUserPatch
    // pub async fn patch<T: AsRef<str>>(&self, id: T, operation: ScimPatchOp) -> Result<ScimUser, ScimError> {
    //     unimplemented!()
    // }
}

pub struct AirtableScimGroupClient {
    inner: Inner,
}

impl AirtableScimGroupClient {
    fn singular_endpoint() -> &'static str {
        "https://airtable.com/scim/v2/Group"
    }

    fn plural_endpoint() -> &'static str {
        "https://airtable.com/scim/v2/Groups"
    }

    fn url(base: &str, path: Option<&str>) -> Result<Url, ScimError> {
        let url = Url::parse(base)?;

        if let Some(path) = path {
            Ok(url.join("/")?.join(path)?)
        } else {
            Ok(url)
        }
    }

    /// From: https://airtable.com/api/enterprise#scimGroupsList
    pub async fn list(&self) -> Result<ScimListResponse<ScimGroupIndex>, ScimError> {
        let req = self
            .inner
            .request(Method::GET, Self::url(Self::plural_endpoint(), None)?, None)?
            .body("")
            .build()?;
        let resp = self.inner.execute(req).await?;

        to_client_response(resp).await
    }

    /// From: https://airtable.com/api/enterprise#scimGroupsGetById
    pub async fn get<T: AsRef<str>>(&self, id: T) -> Result<Option<ScimGroup>, ScimError> {
        let req = self
            .inner
            .request(
                Method::GET,
                Self::url(Self::plural_endpoint(), Some(id.as_ref()))?,
                None,
            )?
            .body("")
            .build()?;
        let resp = self.inner.execute(req).await?;

        to_client_response(resp).await
    }

    /// From: https://airtable.com/api/enterprise#scimGroupCreate
    pub async fn create(&self, new_group: &ScimCreateGroup) -> Result<ScimWriteGroupResponse, ScimError> {
        let req = self
            .inner
            .request(Method::POST, Self::url(Self::singular_endpoint(), None)?, None)?
            .body(serde_json::to_string(new_group)?)
            .build()?;
        let resp = self.inner.execute(req).await?;

        to_client_response(resp).await
    }

    /// From: https://airtable.com/api/enterprise#scimGroupUpdate
    pub async fn update<T: AsRef<str>>(
        &self,
        id: T,
        group: &ScimUpdateGroup,
    ) -> Result<ScimWriteGroupResponse, ScimError> {
        let req = self
            .inner
            .request(
                Method::PUT,
                Self::url(Self::singular_endpoint(), Some(id.as_ref()))?,
                None,
            )?
            .body(serde_json::to_string(group)?)
            .build()?;
        let resp = self.inner.execute(req).await?;

        to_client_response(resp).await
    }

    // /// From: https://airtable.com/api/enterprise#scimGroupPatch
    // pub async fn patch<T: AsRef<str>>(&self, id: T, operation: ScimPatchOp) -> Result<ScimGroup, ScimError> {
    //     unimplemented!()
    // }

    /// From: https://airtable.com/api/enterprise#scimGroupDelete
    pub async fn delete<T: AsRef<str>>(&self, id: T) -> Result<(), ScimError> {
        let req = self
            .inner
            .request(
                Method::DELETE,
                Self::url(Self::plural_endpoint(), Some(id.as_ref()))?,
                None,
            )?
            .body("")
            .build()?;
        let resp = self.inner.execute(req).await?;

        // Delete does not return a body on success
        if resp.status() == StatusCode::OK {
            Ok(())
        } else {
            to_client_response(resp).await
        }
    }
}

async fn to_client_response<T>(response: Response) -> Result<T, ScimError>
where
    T: DeserializeOwned,
{
    let status = response.status().clone();

    if status == StatusCode::OK {
        let data: T = response.json().await?;
        Ok(data)
    } else if status == StatusCode::UNAUTHORIZED {
        let error: AirtableEnterpriseError = response.json().await?;
        Err(ScimError::Api(AirtableScimError {
            schemas: vec![],
            status: status.as_u16(),
            detail: error.error.message,
        }))
    } else {
        // Capture SCIM errors
        let error: AirtableScimError = response.json().await?;
        Err(ScimError::Api(error))
    }
}

#[derive(Debug)]
pub enum ScimError {
    Airtable(AirtableError),
    Api(AirtableScimError),
    Client(ClientError),
}

#[derive(Debug)]
pub struct ClientError {
    pub error: Box<dyn std::error::Error>,
}

impl From<AirtableError> for ScimError {
    fn from(err: AirtableError) -> Self {
        Self::Airtable(err)
    }
}

impl From<reqwest::Error> for ScimError {
    fn from(err: reqwest::Error) -> Self {
        Self::Client(ClientError { error: Box::new(err) })
    }
}

impl From<url::ParseError> for ScimError {
    fn from(err: url::ParseError) -> Self {
        Self::Client(ClientError { error: Box::new(err) })
    }
}

impl From<serde_json::Error> for ScimError {
    fn from(err: serde_json::Error) -> Self {
        Self::Client(ClientError { error: Box::new(err) })
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct AirtableScimError {
    schemas: Vec<String>,
    status: u16,
    detail: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimListResponse<T> {
    pub schemas: Vec<String>,
    #[serde(rename = "totalResults")]
    pub total_results: u32,
    #[serde(rename = "startIndex")]
    pub start_index: u32,
    #[serde(rename = "Resources")]
    pub resources: Vec<T>,
    #[serde(rename = "itemsPerPage")]
    pub items_per_page: u32,
}

#[derive(Debug, PartialEq, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimUser {
    pub schemas: Vec<String>,
    pub id: String,
    #[serde(rename = "userName")]
    pub username: String,
    pub name: ScimName,
    pub active: bool,
    pub meta: ScimUserMeta,
    pub emails: Vec<ScimUserEmail>,
    #[serde(flatten)]
    pub extensions: HashMap<String, HashMap<String, Value>>,
}

#[derive(Debug, PartialEq, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimName {
    #[serde(rename = "familyName")]
    pub family_name: String,
    #[serde(rename = "givenName")]
    pub given_name: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimUserMeta {
    pub created: DateTime<Utc>,
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    pub location: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimUserEmail {
    pub value: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimCreateUser {
    schemas: Vec<String>,
    #[serde(rename = "userName")]
    user_name: String,
    name: ScimName,
    /// The title field is available in create and update requests, but it is not returned in
    /// retrieval responses
    /// See: https://airtable.com/api/enterprise#scimUserFieldTypes
    title: String,
    #[serde(flatten)]
    extensions: HashMap<String, HashMap<String, Value>>,
}

#[derive(Debug, PartialEq, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimUpdateUser {
    schemas: Option<Vec<String>>,
    #[serde(rename = "userName")]
    user_name: Option<String>,
    name: Option<ScimName>,
    /// The title field is available in create and update requests, but it is not returned in
    /// retrieval responses
    /// See: https://airtable.com/api/enterprise#scimUserFieldTypes
    title: Option<String>,
    active: Option<bool>,
    #[serde(flatten)]
    extensions: Option<HashMap<String, HashMap<String, Value>>>,
}

#[derive(Debug, PartialEq, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimGroupIndex {
    pub schemas: Vec<String>,
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, PartialEq, Default, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimGroup {
    pub schemas: Vec<String>,
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub members: Vec<ScimGroupMember>,
}

#[derive(Debug, PartialEq, Default, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimGroupMember {
    pub value: String,
}

#[derive(Debug, PartialEq, Default, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimCreateGroup {
    pub schemas: Vec<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[derive(Debug, PartialEq, Default, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimUpdateGroup {
    pub schemas: Option<Vec<String>>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub members: Option<Vec<ScimGroupMember>>,
}

#[derive(Debug, PartialEq, Default, Clone, Serialize, JsonSchema, Deserialize)]
pub struct ScimWriteGroupResponse {
    pub schemas: Vec<String>,
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use http::{header::HeaderValue, Response as HttpResponse, StatusCode};
    use reqwest::{Client, Method, Request, Response};
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, RequestBuilder};
    use serde_json::{Map, Value};
    use std::sync::Arc;

    use super::*;
    use crate::{error::AirtableError, inner::ApiClient};

    struct MockClient<T> {
        exec: T,
        response: String,
        client: ClientWithMiddleware,
    }

    #[async_trait]
    impl<T> ApiClient for MockClient<T>
    where
        T: Fn(Request) -> Option<Response> + Send + Sync,
    {
        fn key(&self) -> &str {
            unimplemented!()
        }

        fn base_id(&self) -> &str {
            unimplemented!()
        }

        fn enterprise_account_id(&self) -> &str {
            unimplemented!()
        }

        fn client(&self) -> &reqwest_middleware::ClientWithMiddleware {
            unimplemented!()
        }

        fn request(
            &self,
            method: Method,
            url: Url,
            _query: Option<Vec<(&str, String)>>,
        ) -> Result<RequestBuilder, AirtableError> {
            let rb = self.client.request(method.clone(), url);

            Ok(rb)
        }

        async fn execute(&self, request: Request) -> Result<Response, AirtableError> {
            let handler_resp = (self.exec)(request);

            if let Some(handler_resp) = handler_resp {
                Ok(handler_resp)
            } else {
                let mut response = HttpResponse::new(self.response.clone());
                response
                    .headers_mut()
                    .insert("Content-Type", HeaderValue::from_static("application/json"));
                *response.status_mut() = StatusCode::OK;

                Ok(response.into())
            }
        }
    }

    fn make_client<T>(exec: T, response: &str) -> AirtableScimClient
    where
        T: Fn(Request) -> Option<Response> + Send + Sync + 'static,
    {
        let reqwest_client = Client::builder().build().unwrap();

        let mock = MockClient {
            exec,
            response: response.to_string(),
            client: ClientBuilder::new(reqwest_client).build(),
        };

        AirtableScimClient { inner: Arc::new(mock) }
    }

    fn ok_client(response: &str) -> AirtableScimClient {
        make_client(|_req| None, response)
    }

    #[tokio::test]
    async fn test_unauthorized() {
        let client = make_client(
            |_req| {
                let mut resp = HttpResponse::new(
                    r#"{
    "error": {
        "type": "AUTHENTICATION_REQUIRED",
        "message": "Authentication required"
    }
}"#,
                );
                *resp.status_mut() = StatusCode::UNAUTHORIZED;

                Some(resp.into())
            },
            "",
        );

        let resp = client.user().list().await;

        match resp {
            Err(ScimError::Api(AirtableScimError {
                schemas,
                status,
                detail,
            })) => {
                assert_eq!(Vec::<String>::new(), schemas);
                assert_eq!(StatusCode::UNAUTHORIZED, status);
                assert_eq!("Authentication required".to_string(), detail);
            }
            other => panic!("Received non-unauthorized response {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_list_users_ok() {
        let client = ok_client(
            r#"{
    "schemas": [
        "urn:ietf:params:scim:api:messages:2.0:ListResponse"
    ],
    "totalResults": 3,
    "startIndex": 2,
    "Resources": [
        {
        "schemas": [
            "urn:ietf:params:scim:schemas:core:2.0:User"
        ],
        "id": "usr00000000000000",
        "userName": "foo@bar.com",
        "name": {
            "familyName": "Jane",
            "givenName": "Doe"
        },
        "active": true,
        "meta": {
            "created": "2021-06-02T07:37:19.000Z",
            "resourceType": "User",
            "location": "/scim/v2/Users/usr00000000000000"
        },
        "emails": [
            {
            "value": "foo@bar.com"
            }
        ]
        }
    ],
    "itemsPerPage": 1
}"#,
        );

        let users = client.user().list().await.unwrap();

        let expected = ScimListResponse {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:ListResponse".to_string()],
            total_results: 3,
            start_index: 2,
            resources: vec![ScimUser {
                schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:User".to_string()],
                id: "usr00000000000000".to_string(),
                username: "foo@bar.com".to_string(),
                name: ScimName {
                    family_name: "Jane".to_string(),
                    given_name: "Doe".to_string(),
                },
                active: true,
                meta: ScimUserMeta {
                    created: DateTime::parse_from_rfc3339("2021-06-02T07:37:19Z")
                        .map(|fixed| fixed.with_timezone(&Utc))
                        .unwrap(),
                    resource_type: "User".to_string(),
                    location: "/scim/v2/Users/usr00000000000000".to_string(),
                },
                emails: vec![ScimUserEmail {
                    value: "foo@bar.com".to_string(),
                }],
                extensions: HashMap::new(),
            }],
            items_per_page: 1,
        };

        assert_eq!(expected, users);
    }

    #[tokio::test]
    async fn test_get_user_ok() {
        let client = ok_client(
            r#"{
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:User"
    ],
    "id": "usr00000000000000",
    "userName": "foo@bar.com",
    "name": {
        "familyName": "Jane",
        "givenName": "Doe"
    },
    "active": true,
    "meta": {
        "created": "2021-06-02T07:37:19.000Z",
        "resourceType": "User",
        "location": "/scim/v2/Users/usr00000000000000"
    },
    "emails": [
        {
        "value": "foo@bar.com"
        }
    ]
}
"#,
        );

        let user = client.user().get("usr00000000000000").await.unwrap();

        let expected = Some(ScimUser {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:User".to_string()],
            id: "usr00000000000000".to_string(),
            username: "foo@bar.com".to_string(),
            name: ScimName {
                family_name: "Jane".to_string(),
                given_name: "Doe".to_string(),
            },
            active: true,
            meta: ScimUserMeta {
                created: DateTime::parse_from_rfc3339("2021-06-02T07:37:19Z")
                    .map(|fixed| fixed.with_timezone(&Utc))
                    .unwrap(),
                resource_type: "User".to_string(),
                location: "/scim/v2/Users/usr00000000000000".to_string(),
            },
            emails: vec![ScimUserEmail {
                value: "foo@bar.com".to_string(),
            }],
            extensions: HashMap::new(),
        });

        assert_eq!(expected, user);
    }

    #[test]
    fn test_create_user_ser() {
        let mut extensions = HashMap::new();
        let mut user_ext = HashMap::new();

        user_ext.insert(
            "costCenter".to_string(),
            Value::String("Example cost center".to_string()),
        );
        user_ext.insert(
            "department".to_string(),
            Value::String("Example department".to_string()),
        );
        user_ext.insert("division".to_string(), Value::String("Example division".to_string()));
        user_ext.insert(
            "organization".to_string(),
            Value::String("Example organization".to_string()),
        );

        let mut manager_map = Map::new();
        manager_map.insert("displayName".to_string(), Value::String("John Doe".to_string()));
        manager_map.insert("value".to_string(), Value::String("foo@bam.com".to_string()));
        user_ext.insert("manager".to_string(), Value::Object(manager_map));

        extensions.insert(
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
            user_ext,
        );

        let create_user = ScimCreateUser {
            schemas: vec![
                "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
                "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
            ],
            user_name: "foo@bar.com".to_string(),
            name: ScimName {
                family_name: "Jane".to_string(),
                given_name: "Doe".to_string(),
            },
            title: "Manager".to_string(),
            extensions,
        };

        let expected: ScimCreateUser = serde_json::from_str(
            r#"{
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:User",
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
    ],
    "userName": "foo@bar.com",
    "name": {
        "familyName": "Jane",
        "givenName": "Doe"
    },
    "title": "Manager",
    "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
        "costCenter": "Example cost center",
        "department": "Example department",
        "division": "Example division",
        "organization": "Example organization",
        "manager": {
            "displayName": "John Doe",
            "value": "foo@bam.com"
        }
    }
}"#,
        )
        .unwrap();

        assert_eq!(
            expected,
            serde_json::from_str(&serde_json::to_string(&create_user).unwrap()).unwrap()
        );
    }

    #[tokio::test]
    async fn test_create_users_ok() {
        let client = ok_client(r#"{
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:User"
    ],
    "id": "usr00000000000000",
    "userName": "foo@bar.com",
    "name": {
        "familyName": "Jane",
        "givenName": "Doe"
    },
    "active": true,
    "meta": {
        "created": "2021-06-02T07:37:19.000Z",
        "resourceType": "User",
        "location": "/scim/v2/Users/usr00000000000000"
    },
    "emails": [
        {
            "value": "foo@bar.com"
        }
    ]
}"#);

        let mut extensions = HashMap::new();
        let mut user_ext = HashMap::new();

        user_ext.insert(
            "costCenter".to_string(),
            Value::String("Example cost center".to_string()),
        );
        user_ext.insert(
            "department".to_string(),
            Value::String("Example department".to_string()),
        );
        user_ext.insert("division".to_string(), Value::String("Example division".to_string()));
        user_ext.insert(
            "organization".to_string(),
            Value::String("Example organization".to_string()),
        );

        let mut manager_map = Map::new();
        manager_map.insert("displayName".to_string(), Value::String("John Doe".to_string()));
        manager_map.insert("value".to_string(), Value::String("foo@bam.com".to_string()));
        user_ext.insert("manager".to_string(), Value::Object(manager_map));

        extensions.insert(
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
            user_ext,
        );

        let create_user = ScimCreateUser {
            schemas: vec![
                "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
                "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
            ],
            user_name: "foo@bar.com".to_string(),
            name: ScimName {
                family_name: "Jane".to_string(),
                given_name: "Doe".to_string(),
            },
            title: "Manager".to_string(),
            extensions,
        };

        let user = client.user().create(&create_user).await.unwrap();

        let expected = ScimUser {
            schemas: vec![
                "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
            ],
            id: "usr00000000000000".to_string(),
            username: "foo@bar.com".to_string(),
            name: ScimName {
                family_name: "Jane".to_string(),
                given_name: "Doe".to_string(),
            },
            active: true,
            meta: ScimUserMeta {
                created: DateTime::parse_from_rfc3339("2021-06-02T07:37:19Z")
                    .map(|fixed| fixed.with_timezone(&Utc))
                    .unwrap(),
                resource_type: "User".to_string(),
                location: "/scim/v2/Users/usr00000000000000".to_string(),
            },
            emails: vec![
                ScimUserEmail {
                    value: "foo@bar.com".to_string(),
                },
            ],
            extensions: HashMap::new(),
        };

        assert_eq!(expected, user);
    }

    #[test]
    fn test_update_user_ser() {
        let mut extensions = HashMap::new();
        let mut user_ext = HashMap::new();

        user_ext.insert(
            "costCenter".to_string(),
            Value::String("Example cost center".to_string()),
        );
        user_ext.insert(
            "department".to_string(),
            Value::String("Example department".to_string()),
        );
        user_ext.insert("division".to_string(), Value::String("Example division".to_string()));
        user_ext.insert(
            "organization".to_string(),
            Value::String("Example organization".to_string()),
        );

        let mut manager_map = Map::new();
        manager_map.insert("displayName".to_string(), Value::String("John Doe".to_string()));
        manager_map.insert("value".to_string(), Value::String("foo@bam.com".to_string()));
        user_ext.insert("manager".to_string(), Value::Object(manager_map));

        extensions.insert(
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
            user_ext,
        );

        let update_user = ScimUpdateUser {
            schemas: Some(vec![
                "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
                "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
            ]),
            user_name: Some("foo@bar.com".to_string()),
            name: Some(ScimName {
                family_name: "Jane".to_string(),
                given_name: "Doe".to_string(),
            }),
            title: Some("Manager".to_string()),
            active: Some(false),
            extensions: Some(extensions),
        };

        let expected: ScimUpdateUser = serde_json::from_str(
            r#"{
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:User",
        "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User"
    ],
    "userName": "foo@bar.com",
    "name": {
        "familyName": "Jane",
        "givenName": "Doe"
    },
    "title": "Manager",
    "active": false,
    "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User": {
        "costCenter": "Example cost center",
        "department": "Example department",
        "division": "Example division",
        "organization": "Example organization",
        "manager": {
            "displayName": "John Doe",
            "value": "foo@bam.com"
        }
    }
}"#).unwrap();

        assert_eq!(
            expected,
            serde_json::from_str(&serde_json::to_string(&update_user).unwrap()).unwrap()
        );
    }

    #[tokio::test]
    async fn test_update_users_ok() {
        let client = ok_client(r#"{
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:User"
    ],
    "id": "usr00000000000000",
    "userName": "foo@bar.com",
    "name": {
        "familyName": "Jane",
        "givenName": "Doe"
    },
    "active": false,
    "meta": {
        "created": "2021-06-02T07:37:19.000Z",
        "resourceType": "User",
        "location": "/scim/v2/Users/usr00000000000000"
    },
    "emails": [
        {
            "value": "foo@bar.com"
        }
    ]
}"#);

        let mut extensions = HashMap::new();
        let mut user_ext = HashMap::new();

        user_ext.insert(
            "costCenter".to_string(),
            Value::String("Example cost center".to_string()),
        );
        user_ext.insert(
            "department".to_string(),
            Value::String("Example department".to_string()),
        );
        user_ext.insert("division".to_string(), Value::String("Example division".to_string()));
        user_ext.insert(
            "organization".to_string(),
            Value::String("Example organization".to_string()),
        );

        let mut manager_map = Map::new();
        manager_map.insert("displayName".to_string(), Value::String("John Doe".to_string()));
        manager_map.insert("value".to_string(), Value::String("foo@bam.com".to_string()));
        user_ext.insert("manager".to_string(), Value::Object(manager_map));

        extensions.insert(
            "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
            user_ext,
        );

        let update_user = ScimUpdateUser {
            schemas: Some(vec![
                "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
                "urn:ietf:params:scim:schemas:extension:enterprise:2.0:User".to_string(),
            ]),
            user_name: Some("foo@bar.com".to_string()),
            name: Some(ScimName {
                family_name: "Jane".to_string(),
                given_name: "Doe".to_string(),
            }),
            title: Some("Manager".to_string()),
            active: Some(false),
            extensions: Some(extensions),
        };

        let user = client.user().update("usr00000000000000", &update_user).await.unwrap();

        let expected = ScimUser {
            schemas: vec![
                "urn:ietf:params:scim:schemas:core:2.0:User".to_string(),
            ],
            id: "usr00000000000000".to_string(),
            username: "foo@bar.com".to_string(),
            name: ScimName {
                family_name: "Jane".to_string(),
                given_name: "Doe".to_string(),
            },
            active: false,
            meta: ScimUserMeta {
                created: DateTime::parse_from_rfc3339("2021-06-02T07:37:19Z")
                    .map(|fixed| fixed.with_timezone(&Utc))
                    .unwrap(),
                resource_type: "User".to_string(),
                location: "/scim/v2/Users/usr00000000000000".to_string(),
            },
            emails: vec![
                ScimUserEmail {
                    value: "foo@bar.com".to_string(),
                },
            ],
            extensions: HashMap::new(),
        };

        assert_eq!(expected, user);
    }

    #[tokio::test]
    async fn test_list_groups_ok() {
        let client = ok_client(
            r#"{
    "schemas": [
        "urn:ietf:params:scim:api:messages:2.0:ListResponse"
    ],
    "totalResults": 1,
    "startIndex": 1,
    "Resources": [
        {
            "schemas": [
                "urn:ietf:params:scim:schemas:core:2.0:Group"
            ],
            "id": "ugpQ7PJ2boxzMAKFU",
            "displayName": "ExampleGroup"
        }
    ],
    "itemsPerPage": 1
}"#,
        );

        let groups = client.group().list().await.unwrap();

        let expected = ScimListResponse {
            schemas: vec!["urn:ietf:params:scim:api:messages:2.0:ListResponse".to_string()],
            total_results: 1,
            start_index: 1,
            resources: vec![ScimGroupIndex {
                schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
                id: "ugpQ7PJ2boxzMAKFU".to_string(),
                display_name: "ExampleGroup".to_string(),
            }],
            items_per_page: 1,
        };

        assert_eq!(expected, groups);
    }

    #[tokio::test]
    async fn test_get_group_ok() {
        let client = ok_client(
            r#"{
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:Group"
    ],
    "id": "ugpQ7PJ2boxzMAKFU",
    "displayName": "ExampleGroup",
    "members": [
        {
            "value": "usrI7HMkO7sAefUHk"
        },
        {
            "value": "usrM4UuTPOjRlDOHT"
        }
    ]
}"#,
        );

        let group = client.group().get("ugpQ7PJ2boxzMAKFU").await.unwrap();

        let expected = Some(ScimGroup {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
            id: "ugpQ7PJ2boxzMAKFU".to_string(),
            display_name: "ExampleGroup".to_string(),
            members: vec![
                ScimGroupMember {
                    value: "usrI7HMkO7sAefUHk".to_string(),
                },
                ScimGroupMember {
                    value: "usrM4UuTPOjRlDOHT".to_string(),
                },
            ],
        });

        assert_eq!(expected, group);
    }

    #[tokio::test]
    async fn test_create_group_ok() {
        let client = ok_client(
            r#"{
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:Group"
    ],
    "id": "ugpEOS67LautSwEKM",
    "displayName": "ExampleGroup"
}"#,
        );

        let created = client
            .group()
            .create(&ScimCreateGroup {
                schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
                display_name: "ExampleGroup".to_string(),
            })
            .await
            .unwrap();

        let expected = ScimWriteGroupResponse {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
            id: "ugpEOS67LautSwEKM".to_string(),
            display_name: "ExampleGroup".to_string(),
        };

        assert_eq!(expected, created);
    }

    #[tokio::test]
    async fn test_update_group_ok() {
        let client = ok_client(
            r#"{
    "schemas": [
        "urn:ietf:params:scim:schemas:core:2.0:Group"
    ],
    "id": "ugpEOS67LautSwEKM",
    "displayName": "Updated Example Group"
}"#,
        );

        let updated = client
            .group()
            .update(
                "ugpEOS67LautSwEKM",
                &ScimUpdateGroup {
                    schemas: Some(vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()]),
                    display_name: Some("Updated Example Group".to_string()),
                    members: Some(vec![ScimGroupMember {
                        value: "test@user.com".to_string(),
                    }]),
                },
            )
            .await
            .unwrap();

        let expected = ScimWriteGroupResponse {
            schemas: vec!["urn:ietf:params:scim:schemas:core:2.0:Group".to_string()],
            id: "ugpEOS67LautSwEKM".to_string(),
            display_name: "Updated Example Group".to_string(),
        };

        assert_eq!(expected, updated);
    }

    #[tokio::test]
    async fn test_delete_group_ok() {
        let client = ok_client(r#""#);

        let empty = client.group().delete("ugpQ7PJ2boxzMAKFU").await.unwrap();

        assert_eq!((), empty);
    }
}