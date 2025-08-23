use std::{collections::HashMap, sync::{Arc, Mutex}};

use actix_web::{get, web, HttpResponse, Responder};
use moka::future::Cache;
use regex::Regex;
use scraper::{Html, Selector};

use crate::Response;

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
