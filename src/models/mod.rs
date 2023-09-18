use std::{collections::HashMap, time::SystemTime};

use serde::{Deserialize, Serialize};

use crate::{
    events::{Event, Restore},
    tweet_controller::{Diff, JsonString, ToJson},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tweet {
    pub id: String,
    pub content: String,
    pub user_id: String,
    pub timestamp: u64,
}

impl Tweet {
    pub fn new(content: String, user_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            user_id,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

impl Diff for Tweet {
    fn diff(&self, other: &Self) -> Result<JsonString, String> {
        let mut diff: HashMap<String, serde_json::Value> = HashMap::new();

        if self.id != other.id {
            diff.insert(
                "id".into(),
                serde_json::to_value(other.id.clone())
                    .map_err(|err| format!("Failed to serialize id: {}", err))?,
            );
        }

        if self.content != other.content {
            diff.insert(
                "content".into(),
                serde_json::to_value(other.content.clone())
                    .map_err(|err| format!("Failed to serialize content: {}", err))?,
            );
        }

        if self.user_id != other.user_id {
            diff.insert(
                "user_id".into(),
                serde_json::to_value(other.user_id.clone())
                    .map_err(|err| format!("Failed to serialize user_id: {}", err))?,
            );
        }

        if self.timestamp != other.timestamp {
            diff.insert(
                "timestamp".into(),
                serde_json::to_value(other.timestamp)
                    .map_err(|err| format!("Failed to serialize timestamp: {}", err))?,
            );
        }

        serde_json::to_value(diff)
            .map_err(|err| format!("Failed to serialize diff: {}", err))
            .map(|json| json.to_string())
    }
}

impl Restore for Tweet {
    type T = Self;
    fn restore(&mut self, event: &Event) -> Result<&mut Self, String> {
        let json = serde_json::from_str::<serde_json::Value>(&event.json.clone())
            .map_err(|err| format!("Failed to deserialize event: {}", err))?;

        let json = json
            .as_object()
            .ok_or("Failed to get json object".to_string())?;

        let id = json.get("id").map(|id| id.as_str()).unwrap_or(None);
        if let Some(id) = id {
            self.id = id.to_string();
        };

        let content = json
            .get("content")
            .map(|content| content.as_str())
            .unwrap_or(None);
        if let Some(content) = content {
            self.content = content.to_string();
        };

        let user_id = json
            .get("user_id")
            .map(|user_id| user_id.as_str())
            .unwrap_or(None);
        if let Some(user_id) = user_id {
            self.user_id = user_id.to_string();
        };

        let timestamp = json
            .get("timestamp")
            .map(|timestamp| timestamp.as_u64())
            .unwrap_or(None);
        if let Some(timestamp) = timestamp {
            self.timestamp = timestamp;
        };

        Ok(self)
    }
}

impl ToJson for Tweet {
    fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&self).map_err(|err| format!("Failed to serialize tweet: {}", err))
    }
}

impl Default for Tweet {
    fn default() -> Self {
        Self {
            id: "".into(),
            content: "".into(),
            user_id: "".into(),
            timestamp: 0,
        }
    }
}

pub struct SledIVec(pub sled::IVec);
impl TryInto<Vec<Tweet>> for SledIVec {
    type Error = String;
    fn try_into(self) -> Result<Vec<Tweet>, Self::Error> {
        let Ok(tweet) = serde_json::from_slice::<Vec<Tweet>>(&self.0) else {
            return Err("Failed to deserialize tweet".to_string());
        };
        Ok(tweet)
    }
}
