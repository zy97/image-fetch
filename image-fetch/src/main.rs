mod caoliu;
mod heiliao;
mod mrds;
use axum::{Router, routing::get};
use bb8_redis::{
    RedisConnectionManager,
    bb8::{self, Pool},
    redis::AsyncCommands,
};
use caoliu::caoliu as cl_route;
use caoliu::caoliu_image;
use heiliao::hl;
use mrds::mrds as mrds_route;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt};
#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct Response {
    pub title: String,
    pub images: Vec<String>,
    pub videos: Vec<String>,
}
#[derive(Deserialize)]
pub struct ChiGuaServer {
    url: String,
}
type ConnectionPool = Pool<RedisConnectionManager>;
#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::debug!("connecting to redis");
    let manager = RedisConnectionManager::new("redis://localhost").unwrap();
    let pool = bb8::Pool::builder().build(manager).await.unwrap();

    {
        // ping the database before starting
        let mut conn = pool.get().await.unwrap();
        conn.set::<&str, &str, ()>("foo", "bar").await.unwrap();
        let result: String = conn.get("foo").await.unwrap();
        assert_eq!(result, "bar");
    }
    tracing::debug!("successfully connected to redis and pinged it");

    // build our application with a route
    let app = Router::new()
        .route("/mrds/{id}", get(mrds_route))
        .route("/hl/{id}", get(hl))
        .route("/caoliu/{*id}", get(cl_route))
        .route("/caoliu-image", get(caoliu_image))
        .with_state(pool);

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:17619")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
