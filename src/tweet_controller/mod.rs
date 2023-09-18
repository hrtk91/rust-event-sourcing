use std::time::SystemTime;

use axum::Json;

use crate::{dtos, events, models};
pub type JsonString = String;
pub trait ToJson {
    fn to_json(&self) -> Result<JsonString, String>;
}

pub trait Diff {
    fn diff(&self, other: &Self) -> Result<JsonString, String>;
}

pub async fn all() -> Result<String, String> {
    let tweets = {
        let repo = events::EventRepository::new();
        let mut results: Vec<models::Tweet> = repo.restore_all("Tweets").unwrap_or(vec![]);
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        results
    };
    serde_json::to_string(&tweets).map_err(|err| format!("Failed to serialize tweet: {}", err))
}

pub async fn create(Json(tweet): Json<dtos::CreateTweet>) -> Result<String, String> {
    let mut tweets: Vec<models::Tweet> = {
        let db = sled::open("./db")
            .map_err(|err| format!("[create]Failed to open database: {}", err))?;
        db.get("Tweets".as_bytes())
            .map_err(|err| format!("Failed open Tweets: {}", err))?
            .map_or(vec![], |tweets| {
                models::SledIVec(tweets).try_into().unwrap_or_default()
            })
    };

    let new_tweet = models::Tweet::new(tweet.content, tweet.user_id);
    tweets.push(new_tweet.clone());

    events::Event::publish(events::PublishProps::Props {
        r#type: "Tweets".into(),
        aggregate_id: new_tweet.id.clone(),
        json: new_tweet.to_json().map_err(|err| format!("Failed to serialize tweet: {}", err))?,
    })?;

    {
        let db = sled::open("./db")
            .map_err(|err| format!("[create]Failed to open database: {}", err))?;
        if let Err(err) = db.insert(
            "Tweets".as_bytes(),
            serde_json::to_string(&tweets).unwrap().as_bytes(),
        ) {
            return Err(format!("Failed to insert tweet: {}", err));
        };
    }

    let Ok(new_tweet) = serde_json::to_string(&new_tweet) else {
        return Err("Failed to serialize tweet".to_string());
    };

    Ok(new_tweet)
}

pub async fn update(Json(dto): Json<dtos::UpdateTweet>) -> Result<String, String> {
    let mut tweets: Vec<models::Tweet> = {
        let db = sled::open("./db")
            .map_err(|err| format!("[create]Failed to open database: {}", err))?;
        db.get("Tweets".as_bytes())
            .map_err(|err| format!("Failed open Tweets: {}", err))?
            .map_or(Ok(vec![]), |tweets| models::SledIVec(tweets).try_into())
            .unwrap_or(vec![])
    };

    let tweet_json = {
        let tweet = tweets
            .iter_mut()
            .find(|t| t.id == dto.id)
            .ok_or("Tweet not found".to_string())?;
        let old_tweet = tweet.clone();
        tweet.content = dto.content.clone();
        tweet.timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        events::Event::publish(events::PublishProps::Props {
            r#type: "Tweets".into(),
            aggregate_id: tweet.id.clone(),
            json: old_tweet
                .diff(tweet)
                .map_err(|err| format!("Failed to diff tweet: {}", err))?,
        })?;

        tweet.to_json()?
    };

    {
        let db = sled::open("./db")
            .map_err(|err| format!("[update]Failed to open database: {}", err))?;
        db.insert(
            "Tweets".as_bytes(),
            serde_json::to_string(&tweets).unwrap().as_bytes(),
        )
        .map_err(|err| format!("Failed to insert tweet: {}", err))?;
    }

    Ok(tweet_json)
}

pub async fn delete(tweet_id: String) -> Result<(), String> {
    let tweets = {
        let db = sled::open("./db")
            .map_err(|err| format!("[delete]Failed to open database: {}", err))?;

        db.get("Tweets".as_bytes())
            .map_or(Some(sled::IVec::from(vec![])), |tweets| tweets)
            .map_or(vec![], |tweets| {
                match serde_json::from_slice::<Vec<models::Tweet>>(&tweets) {
                    Ok(tweets) => tweets,
                    Err(_) => Vec::new(),
                }
            })
    };

    let tweets = tweets
        .into_iter()
        .filter(|t| t.id != tweet_id)
        .collect::<Vec<models::Tweet>>();

    {
        let db = sled::open("./db")
            .map_err(|err| format!("[delete]Failed to open database: {}", err))?;

        if let Err(err) = db.insert(
            "Tweets".as_bytes(),
            serde_json::to_string(&tweets).unwrap().as_bytes(),
        ) {
            return Err(format!("Failed to insert tweet: {}", err));
        };
    }

    Ok(())
}
