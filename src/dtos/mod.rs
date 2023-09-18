use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub name: String,
    pub pass: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateTweet {
    pub content: String,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UpdateTweet {
    pub id: String,
    pub content: String,
}
