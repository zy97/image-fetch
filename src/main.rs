use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
mod caoliu;
mod heiliao;
mod mrds;
use caoliu::caoliu as cl_route;
use heiliao::hl;
use mrds::mrds as mrds_route;
mod config;
use actix_web::{App, HttpServer, web};
use config::APP_CONFIG;
use moka::future::Cache;
use serde::Serialize;
const ONE_WEEK_IN_SECONDS: u64 = 60 * 60 * 24 * 7;
#[derive(Debug, Serialize)]
pub struct Response {
    pub images: Vec<String>,
    pub videos: Vec<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 创建配置
    let config: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    {
        let mut config = config.lock().unwrap();
        config.insert("heiliao".to_string(), APP_CONFIG.config.heiliao.clone());
        config.insert(
            "meiridasai".to_string(),
            APP_CONFIG.config.meiridasai.clone(),
        );
        config.insert("caoliu".to_string(), APP_CONFIG.config.caoliu.clone());
        config.insert("url".to_string(), APP_CONFIG.config.url.clone());
    }
    let cache: Cache<String, String> = Cache::builder()
        .time_to_live(Duration::from_secs(ONE_WEEK_IN_SECONDS))
        .build();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(cache.clone()))
            .service(hl)
            .service(mrds_route)
            .service(cl_route)
            .service(caoliu::caoliu_image)
    })
    .bind(("0.0.0.0", 17619))?
    .run()
    .await
}
