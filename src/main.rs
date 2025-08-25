use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
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
use serde_json::Value;
const ONE_WEEK_IN_SECONDS: u64 = 60 * 60 * 24 * 7;
#[derive(Debug, Serialize)]
pub struct Response {
    pub title: String,
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

fn read_json(key_name: &str) -> Option<String> {
    // 读取地址/var/lib/casaos/1/link.json的的文件 获取这个数组里name值等于external的hostname的值
    let file_path = "/var/lib/casaos/1/link.json";
    let file = File::open(file_path).expect("Unable to open file");
    let reader = BufReader::new(file);
    let json: Value = serde_json::from_reader(reader).expect("Unable to parse JSON");
    json.as_array()?
        .iter()
        .filter_map(|item| item.as_object())
        .find(|obj| {
            obj.get("name")
                .and_then(Value::as_str)
                .map_or(false, |name| name == key_name)
        })
        .and_then(|obj| obj.get("hostname"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
}
pub fn read_external() -> Option<String> {
    read_json("external")
}
pub fn read_player() -> Option<String> {
    read_json("dplayer")
}
