use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Mediawiki {
    pub page: Vec<Page>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page {
    pub ns: u64,
    pub id: u64,
    pub title: String,
    pub revision: Revision,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Revision {
    pub id: u64,
    // timestamp: String,
    pub comment: Option<String>,
    pub text: String,
}
