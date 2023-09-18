mod dtos;
mod events;
mod models;
mod tweet_controller;

use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Json, Router,
};

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct User {
    id: String,
    name: String,
    pass: String,
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn create_user(Json(payload): Json<User>) -> Result<String, String> {
    let Ok(db) = sled::open("./db") else {
        return Err("Failed to open database".to_string());
    };

    let mut users = db
        .get("users".as_bytes())
        .map_or(Some(sled::IVec::from(vec![])), |users| users)
        .map_or(vec![], |users| {
            match serde_json::from_slice::<Vec<User>>(&users) {
                Ok(users) => users,
                Err(_) => Vec::new(),
            }
        });

    if let Some(_) = users.iter().find(|user| user.name == payload.name) {
        return Err(format!("User {} already exists", payload.name));
    }

    users.push(payload.clone());

    if let Err(err) = db.insert(
        "users".as_bytes(),
        serde_json::to_string(&users).unwrap().as_bytes(),
    ) {
        return Err(format!("Failed to insert user: {}", err));
    };

    Ok(format!(
        "Hello, {}! Your password is {}",
        payload.name, payload.pass
    ))
}

async fn login(Json(payload): Json<User>) -> Result<String, String> {
    let Ok(db) = sled::open("./db") else {
        return Err("Failed to open database".to_string());
    };

    let Ok(users) = db.get("users".as_bytes()).map(
        |users| match serde_json::from_slice::<Vec<User>>(&users.unwrap()) {
            Ok(users) => users,
            Err(_) => Vec::new()
    }) else {
        return Err("Failed to get users".to_string());
    };

    match users.iter().find(|user| user.name == payload.name) {
        Some(user) => {
            if user.pass == payload.pass {
                Ok(format!("Hello, {}!", payload.name))
            } else {
                Err("Wrong password".to_string())
            }
        }
        None => Err(format!("User {} does not exist", payload.name)),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/create-user", post(create_user))
        .route("/login", post(login))
        .route("/tweets", get(tweet_controller::all))
        .route("/tweet/create", post(tweet_controller::create))
        .route("/tweet/update", post(tweet_controller::update))
        .route("/tweet/delete/:tweet_id", get(tweet_controller::delete))
        .route("/events", get(events::get_events));

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    // tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
