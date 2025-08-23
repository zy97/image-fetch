use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix_web::{HttpResponse, Responder, get, web};
use moka::future::Cache;
use regex::Regex;
use scraper::{Html, Selector};

use crate::Response;

#[get("/mrds/{id}")]
async fn mrds(
    config: web::Data<Arc<Mutex<HashMap<String, String>>>>,
    cache: web::Data<Cache<String, String>>,
    id: web::Path<String>,
) -> impl Responder {
    let key = format!("mrds_{}", id);
    let video_url = cache.get(&key).await;
    match video_url {
        Some(response) => HttpResponse::Ok()
            .content_type("application/json")
            .body(response),
        None => {
            let config = config.lock().unwrap();
            let url = config.get("meiridasai").unwrap();
            let url = format!("{}//archives/{}/", url, id);
            let client = reqwest::Client::new();
            let response = client.get(&url).send().await;
            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let html = resp.text().await.unwrap();
                        let document = Html::parse_document(&html);
                        let selector = Selector::parse(".post-content img").unwrap();

                        let mut image_urls = vec![];
                        for image in document.select(&selector) {
                            let src = image.attr("z-image-loader-url").unwrap();
                            image_urls.push(format!("http://10.144.144.100:9090?image={}", src));
                            println!("匹配的值: {}", src);
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
