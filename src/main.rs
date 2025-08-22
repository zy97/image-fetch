use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
mod config;
use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use config::APP_CONFIG;
use moka::future::Cache;
use regex::Regex;
use reqwest;
use scraper::{Html, Selector};
use serde::Serialize;
use serde_json::Value;
const ONE_WEEK_IN_SECONDS: u64 = 60 * 60 * 24 * 7;
#[derive(Debug, Serialize)]
pub struct Response {
    pub images: Vec<String>,
    pub videos: Vec<String>,
}
#[get("/hl/{id}")]
async fn hl(
    config: web::Data<Arc<Mutex<HashMap<String, String>>>>,
    cache: web::Data<Cache<String, String>>,
    id: web::Path<String>,
) -> impl Responder {
    let key = format!("hl_{}", id);
    let video_url = cache.get(&key).await;
    match video_url {
        Some(response) => HttpResponse::Ok()
            .content_type("application/json")
            .body(response),
        None => {
            let config = config.lock().unwrap();
            let heiliao = config.get("heiliao").unwrap();
            let url = format!("{}//archives/{}/", heiliao, id);
            let client = reqwest::Client::new();
            let response = client.get(&url).send().await;
            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let html = resp.text().await.unwrap();
                        let document = Html::parse_document(&html);
                        let selector = Selector::parse(".client-only-placeholder img").unwrap();

                        let mut image_urls = vec![];
                        for image in document.select(&selector) {
                            let src = image.attr("onload").unwrap();
                            // 正则表达式匹配/loadImg\(this,'(.*?)'\)/ 这个里面的值
                            let re = Regex::new(r"loadImg\(this,'(.*?)'\)").unwrap();
                            // 使用正则表达式查找匹配项
                            if let Some(captures) = re.captures(src) {
                                // 提取第一个分组中的值
                                if let Some(val) = captures.get(1) {
                                    image_urls.push(format!(
                                        "http://10.144.144.100:9090?image={}",
                                        val.as_str()
                                    ));
                                    println!("匹配的值: {}", val.as_str());
                                }
                            } else {
                                println!("未找到匹配");
                            }
                        }

                        let video_url = config.get("url").unwrap();
                        let video_rul = format!("{}/hl/{}", video_url, id);
                        let response = Response {
                            images: image_urls,
                            videos: vec![video_rul],
                        };
                        let response = serde_json::to_string(&response).unwrap();
                        cache.insert(key, response.clone()).await;
                        HttpResponse::Ok()
                            .content_type("application/json")
                            .body(response)
                    } else {
                        HttpResponse::BadRequest().body("Failed to fetch data")
                    }
                }
                Err(_) => HttpResponse::InternalServerError().body("Internal Server Error"),
            }
        }
    }
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
    })
    .bind(("0.0.0.0", 17619))?
    .run()
    .await
}
