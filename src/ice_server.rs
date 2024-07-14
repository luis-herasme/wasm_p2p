use serde;
use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IceServer {
    pub urls: String,
    pub credential: Option<String>,
    pub credential_type: Option<String>,
    pub username: Option<String>,
}

impl From<&str> for IceServer {
    fn from(url: &str) -> IceServer {
        IceServer {
            urls: String::from(url),
            credential: None,
            credential_type: None,
            username: None,
        }
    }
}
