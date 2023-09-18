pub struct EventRepository;
impl EventRepository {
    pub fn new() -> Self {
        Self
    }
    pub fn all(&self) -> Result<Vec<Event>, String> {
        let events = {
            let db = sled::open("./db")
                .map_err(|err| format!("[all]Failed to open database: {}", err))?;
            db.get("Events".as_bytes())
                .map_err(|err| format!("Failed to open Events: {}", err))?
                .map_or(vec![], |events| {
                    serde_json::from_slice::<Vec<Event>>(&events).unwrap_or(vec![])
                })
        };
        Ok(events)
    }
    pub fn create(&self, event: Event) -> Result<(), String> {
        let mut events = {
            let db = sled::open("./db")
                .map_err(|err| format!("[publish]Failed to open database: {}", err))?;

            db.get("Events".as_bytes())
                .map_err(|err| format!("Failed to get Events: {}", err))?
                .map_or(vec![], |events| {
                    serde_json::from_slice::<Vec<Event>>(&events).unwrap_or(vec![])
                })
        };

        events.push(event);

        let events = serde_json::to_string(&events)
            .map_err(|err| format!("Failed to serialize events: {}", err))?;
        {
            let db = sled::open("./db")
                .map_err(|err| format!("[publish]Failed to open database: {}", err))?;
            db.insert("Events".as_bytes(), events.as_bytes())
                .map_err(|err| format!("Failed to insert events: {}", err))?;
        }

        Ok(())
    }

    pub fn restore_all<T>(&self, type_name: &str) -> Result<Vec<T>, String>
    where
        T: Restore + Default,
    {
        let events = self
            .all()
            .map_err(|err| format!("Failed to get events: {}", err))?;
        let mut set = events
            .iter()
            .filter(|event| event.r#type == type_name)
            .fold(
                HashMap::new(),
                |mut acc: HashMap<String, Vec<Event>>, event| {
                    if let Some(events) = acc.get_mut(&event.aggregate_id) {
                        events.push(event.clone())
                    } else {
                        acc.insert(event.aggregate_id.clone(), vec![event.clone()]);
                    }
                    acc
                },
            );
        let results = set.iter_mut().fold(vec![], |mut acc: Vec<T>, (_, events)| {
            events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

            let mut model = T::default();
            events.iter().for_each(|event| {
                model
                    .restore(event)
                    .map_err(|err| format!("Failed to restore tweet: {}", err))
                    .ok();
            });

            acc.push(model);
            acc
        });
        Ok(results)
    }

    pub fn restore<T>(&self, aggregate_id: &str) -> Result<T, String>
    where
        T: Restore + Default,
    {
        let events: Vec<Event> = {
            let db = sled::open("./db")
                .map_err(|err| format!("[get_events]Failed to open database: {}", err))?;
            db.get("Events".as_bytes())
                .map_err(|err| format!("Failed to get Events: {}", err))?
                .map_or(vec![], |events| {
                    serde_json::from_slice::<Vec<Event>>(&events).unwrap_or(vec![])
                })
        };

        let mut events = events
            .iter()
            .filter(|event| event.aggregate_id == aggregate_id)
            .collect::<Vec<&Event>>();
        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let mut result = T::default();
        for event in events.iter() {
            result
                .restore(event)
                .map_err(|err| format!("Failed to restore tweet: {}", err))?;
        }

        Ok(result)
    }
}

pub enum PublishProps {
    Event(Event),
    Props {
        r#type: String,
        aggregate_id: String,
        json: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    pub id: String,
    pub r#type: String,
    pub aggregate_id: String,
    pub json: String,
    pub timestamp: u64,
}
impl Event {
    pub fn new(r#type: &str, aggregate_id: &str, json: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            r#type: r#type.into(),
            aggregate_id: aggregate_id.into(),
            json: json.into(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
    pub fn publish(props: PublishProps) -> Result<(), String> {
        let event = match props {
            PublishProps::Event(event) => event,
            PublishProps::Props {
                r#type,
                aggregate_id,
                json,
            } => Self::new(&r#type, &aggregate_id, &json),
        };

        let repo = EventRepository::new();
        repo.create(event)
    }
}

use std::{collections::HashMap, time::SystemTime};

use serde::{Deserialize, Serialize};

use super::models::SledIVec;
impl TryInto<Vec<Event>> for SledIVec {
    type Error = String;
    fn try_into(self) -> Result<Vec<Event>, Self::Error> {
        serde_json::from_slice::<Vec<Event>>(&self.0)
            .map_err(|err| format!("Failed to deserialize event: {}", err))
    }
}

pub trait Restore {
    type T;
    fn restore(&mut self, event: &Event) -> Result<&mut Self::T, String>;
}

pub async fn get_events() -> Result<String, String> {
    let events = {
        let db = sled::open("./db")
            .map_err(|err| format!("[get_events]Failed to open database: {}", err))?;
        db.get("Events".as_bytes())
            .map_err(|err| format!("Failed to get events: {}", err))?
            .map_or(vec![], |events| -> Vec<Event> {
                SledIVec(events).try_into().unwrap_or(vec![])
            })
    };

    serde_json::to_string(&events).map_err(|err| format!("failed to serialize event: {}", err))
}
