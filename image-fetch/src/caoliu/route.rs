use axum::{
    Json,
    extract::{self, Query, State},
    http::HeaderMap,
    response::IntoResponse,
};
use base64::{Engine, engine::general_purpose};
use bb8_redis::redis::AsyncCommands;
use reqwest::{StatusCode, header::CONTENT_TYPE};
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::Value;

use crate::{ChiGuaServer, ConnectionPool, Response, internal_error};

pub async fn caoliu(
    extract::Path(id): extract::Path<String>,
    State(pool): State<ConnectionPool>,
) -> impl IntoResponse {
    let key = format!("caoliu_{}", id);
    let mut conn = pool.get().await.map_err(internal_error)?;
    let video_url: Option<String> = conn.get(&key).await.map_err(internal_error)?;
    match video_url {
        Some(response) => {
            let json = serde_json::from_str(&response).unwrap();
            Ok((StatusCode::OK, Json(json)))
        }
        None => {
            let response = reqwest::get("http://127.0.0.1:3000/chigua/caoliu").await;
            match response {
                Ok(response) => {
                    let response = response.json::<ChiGuaServer>().await;
                    match response {
                        Ok(response) => {
                            let url = format!("{}/htm_data/{}.html", response.url, id);
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
                                        let response = {
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
                                                    Some(url) => {
                                                        image_urls.push(url.to_string());
                                                        println!("匹配的值: {}", url);
                                                    }
                                                    None => {
                                                        return Err((
                                                            StatusCode::BAD_REQUEST,
                                                            String::from(
                                                                "草榴图片匹配规则以变化有,请更新规则",
                                                            ),
                                                        ));
                                                    }
                                                }
                                            }
                                            let selector =
                                                Selector::parse("#conttpc video").unwrap();
                                            let video_selectors = document.select(&selector);
                                            let mut video_url = String::new();
                                            for video in video_selectors {
                                                if video_url.is_empty() {
                                                    video_url +=
                                                        "https://player.bomky.dpdns.org?items="
                                                }
                                                let video_config = video.value().attr("src");
                                                match video_config {
                                                    Some(url) => video_url += &format!("{url}%&%&"),
                                                    None => {
                                                        return Err((
                                                            StatusCode::BAD_REQUEST,
                                                            String::from(
                                                                "请求草榴视频规则发生了变化",
                                                            ),
                                                        ));
                                                    }
                                                }
                                            }
                                            video_url =
                                                video_url.trim_end_matches("%&%&").to_string();
                                            let mut response = Response {
                                                title,
                                                images: image_urls,
                                                videos: vec![],
                                            };
                                            if !video_url.is_empty() {
                                                response.videos.push(video_url);
                                            }

                                            response
                                        };

                                        conn.set::<String, String, ()>(
                                            key,
                                            serde_json::to_string(&response).unwrap(),
                                        )
                                        .await
                                        .unwrap();
                                        Ok((StatusCode::OK, Json(response)))
                                    } else {
                                        return Err((
                                            StatusCode::BAD_REQUEST,
                                            String::from("请求草榴失败"),
                                        ));
                                    }
                                }
                                Err(_) => {
                                    return Err((
                                        StatusCode::BAD_REQUEST,
                                        String::from("请求草榴失败"),
                                    ));
                                }
                            }
                        }
                        Err(_) => {
                            return Err((
                                StatusCode::BAD_REQUEST,
                                String::from("获取草榴地址成功,但序列化失败"),
                            ));
                        }
                    }
                }
                Err(_) => {
                    return Err((StatusCode::BAD_REQUEST, String::from("获取草榴网地址失败")));
                }
            }
        }
    }
}

pub async fn caoliu_image(
    query: Query<QueryInfo>,
    State(pool): State<ConnectionPool>,
) -> impl IntoResponse {
    let mut conn = pool.get().await.map_err(internal_error)?;
    match query.0.image {
        Some(image_url) => {
            let key = format!("caoliu_{}", image_url);
            let cached_image: Option<String> = conn.get(&key).await.map_err(internal_error)?;
            match cached_image {
                Some(cached_image) => match general_purpose::STANDARD.decode(cached_image) {
                    Ok(image_bytes) => {
                        let mut headers = HeaderMap::new();
                        headers.insert(CONTENT_TYPE, "image/png".parse().unwrap());
                        return Ok((StatusCode::OK, headers, image_bytes));
                    }
                    Err(_) => {
                        return Err((StatusCode::BAD_REQUEST, String::from("解码草榴图片失败")));
                    }
                },
                None => {
                    let response = reqwest::get(image_url).await;
                    match response {
                        Ok(resp) => {
                            if resp.status().is_success() {
                                // 读取图片数据

                                let image_data =
                                    resp.bytes().await.expect("Failed to read image data");
                                let image = image_data.to_vec();
                                let image_base64 = general_purpose::STANDARD.encode(&image);
                                conn.set::<String, String, ()>(key, image_base64.clone())
                                    .await
                                    .unwrap();
                                let mut headers = HeaderMap::new();
                                headers.insert(CONTENT_TYPE, "image/png".parse().unwrap());
                                return Ok((StatusCode::OK, headers, image));
                            } else {
                                return Err((
                                    StatusCode::BAD_REQUEST,
                                    String::from("获取草榴图片失败"),
                                ));
                            }
                        }
                        Err(_) => {
                            return Err((
                                StatusCode::BAD_REQUEST,
                                String::from("获取草榴图片失败"),
                            ));
                        }
                    }
                }
            }
        }
        None => {
            return Err((StatusCode::BAD_REQUEST, String::from("no caoliu image")));
        }
    }
}

#[derive(Deserialize)]
pub struct QueryInfo {
    image: Option<String>,
}
