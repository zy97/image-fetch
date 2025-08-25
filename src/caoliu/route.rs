use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actix_web::{HttpResponse, Responder, get, web};
use base64::{Engine, engine::general_purpose};
use moka::future::Cache;
use scraper::{Html, Selector};

use crate::Response;

#[get("/caoliu/{id:.*}")]
async fn caoliu(
    config: web::Data<Arc<Mutex<HashMap<String, String>>>>,
    cache: web::Data<Cache<String, String>>,
    id: web::Path<String>,
) -> impl Responder {
    let key = format!("caoliu_{}", id);
    let video_url = cache.get(&key).await;
    match video_url {
        Some(response) => HttpResponse::Ok()
            .content_type("application/json")
            .body(response),
        None => {
            let config = config.lock().unwrap();
            let url = config.get("caoliu").unwrap();
            let url = format!("{}/htm_data/{}.html", url, id);
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert("cookie", "ismob=0".parse().unwrap());
            let client = reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .unwrap();
            let response = client.get(&url).send().await;
            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let html = resp.text().await.unwrap();
                        let document = Html::parse_document(&html);
                        let title = document
                            .select(&Selector::parse("title").unwrap())
                            .next()
                            .unwrap()
                            .text()
                            .collect::<String>();
                        let selector = Selector::parse("#conttpc img").unwrap();

                        let mut image_urls = vec![];
                        for image in document.select(&selector) {
                            let src = image.attr("ess-data");
                            match src {
                                Some(url) => image_urls.push(url.to_string()),
                                None => println!("未找到匹配的值"),
                            }
                        }

                        let response = Response {
                            title,
                            images: image_urls,
                            videos: vec![],
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
#[get("/caoliu-image")]
async fn caoliu_image(
    query: web::Query<HashMap<String, String>>,
    cache: web::Data<Cache<String, String>>,
) -> impl Responder {
    let image_url = query.get("image").unwrap();

    let key = format!("caoliu_{}", image_url);
    let video_url = cache.get(&key).await;
    match video_url {
        Some(cached_image) => {
            match general_purpose::STANDARD.decode(cached_image) {
                Ok(image_bytes) => {
                    HttpResponse::Ok()
                        .content_type("image/png") // 或者 image/jpeg 等
                        .body(image_bytes)
                }
                Err(_) => HttpResponse::BadRequest().body("Invalid base64 string"),
            }
        }
        None => {
            let client = reqwest::Client::new();
            let response = client.get(image_url).send().await;
            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        // 读取图片数据

                        let image_data = resp.bytes().await.expect("Failed to read image data");
                        let image = image_data.to_vec();
                        let image = general_purpose::STANDARD.encode(&image);
                        cache.insert(key, image.clone()).await;
                        HttpResponse::Ok()
                            .content_type("image/png")
                            .body(image_data)
                    } else {
                        HttpResponse::BadRequest().body("Failed to fetch data")
                    }
                }
                Err(_) => HttpResponse::InternalServerError().body("Internal Server Error"),
            }
        }
    }
}
