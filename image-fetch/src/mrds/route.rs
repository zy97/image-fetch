use crate::{ChiGuaServer, ConnectionPool, Response, internal_error};
use axum::{
    Json,
    extract::{self, State},
    response::IntoResponse,
};
use bb8_redis::redis::AsyncTypedCommands;
use reqwest::StatusCode;
use scraper::{Html, Selector};
use serde_json::Value;
use tracing::info;

#[axum::debug_handler]
pub async fn mrds(
    extract::Path(id): extract::Path<String>,
    State(pool): State<ConnectionPool>,
) -> impl IntoResponse {
    let key = format!("mrds_{}", id);
    let mut conn = pool.get().await.map_err(internal_error)?;
    let video_url: Option<String> = conn.get(&key).await.map_err(internal_error)?;
    match video_url {
        Some(response) => {
            let json = serde_json::from_str(&response).unwrap();
            Ok((StatusCode::OK, Json(json)))
        }
        None => {
            let response = reqwest::get("http://127.0.0.1:18181/chigua/mrds").await;

            match response {
                Ok(response) => {
                    let response = response.json::<ChiGuaServer>().await;
                    match response {
                        Ok(response) => {
                            let url = format!("{}/archives/{}/", response.url, id);
                            info!("url1: {}", url);
                            let client = reqwest::Client::new();
                            let response = client.get(&url).send().await;
                            info!("url2: {}", url);

                            match response {
                                Ok(resp) => {
                                    info!("status: {}", resp.status());
                                    if resp.status().is_success() {
                                        let html = resp.text().await.unwrap();
                                        let response = {
                                            let document = Html::parse_document(&html);
                                            // conn.set::<&str, &str>("a", "b").await.unwrap();

                                            let title = document
                                                .select(&Selector::parse("title").unwrap())
                                                .next()
                                                .unwrap()
                                                .text()
                                                .collect::<String>();

                                            let selector =
                                                Selector::parse(".post-content img").unwrap();
                                            let host = "https://external.bomky.dpdns.org";
                                            let mut image_urls = vec![];
                                            for image in document.select(&selector) {
                                                let src = image.attr("data-xkrkllgl");
                                                match src {
                                                    Some(src) => {
                                                        image_urls.push(format!(
                                                            "{}/images/?image={}",
                                                            host, src
                                                        ));
                                                        println!("匹配的值: {}", src);
                                                    }
                                                    None => {
                                                        return Err((
                                                            StatusCode::BAD_REQUEST,
                                                            String::from(
                                                                "每日大赛图片路径规则以变化,获取不到",
                                                            ),
                                                        ));
                                                    }
                                                }
                                            }

                                            let selector = Selector::parse(".dplayer").unwrap();
                                            let video_selectors = document.select(&selector);
                                            let mut video_url = String::new();
                                            for video in video_selectors {
                                                if video_url.is_empty() {
                                                    video_url +=
                                                        "https://player.bomky.dpdns.org?items="
                                                }
                                                let video_config =
                                                    video.value().attr("data-config");
                                                match video_config {
                                                    Some(video_config) => {
                                                        //序列化
                                                        let video_config: Value =
                                                            serde_json::from_str(video_config)
                                                                .unwrap();
                                                        let url =
                                                            video_config["video"]["url"].as_str();
                                                        match url {
                                                            Some(url) => {
                                                                video_url += &format!("{url}%&%&")
                                                            }
                                                            None => {
                                                                return Err((
                                                                    StatusCode::BAD_REQUEST,
                                                                    String::from(
                                                                        "请求每日大赛视频规则发生了变化",
                                                                    ),
                                                                ));
                                                            }
                                                        }
                                                    }
                                                    None => {
                                                        return Err((
                                                            StatusCode::BAD_REQUEST,
                                                            String::from(
                                                                "请求每日大赛视频规则发生了变化",
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

                                        conn.set::<&str, String>(
                                            &key,
                                            serde_json::to_string(&response).unwrap(),
                                        )
                                        .await
                                        .unwrap();

                                        Ok((StatusCode::OK, Json(response)))
                                    } else {
                                        return Err((
                                            StatusCode::BAD_REQUEST,
                                            String::from("请求每日大赛失败1"),
                                        ));
                                    }
                                }
                                Err(_) => {
                                    return Err((
                                        StatusCode::BAD_REQUEST,
                                        String::from("请求每日大赛失败2"),
                                    ));
                                }
                            }
                        }
                        Err(_) => {
                            return Err((
                                StatusCode::BAD_REQUEST,
                                String::from("获取每日大赛地址成功,但序列化失败"),
                            ));
                        }
                    }
                }
                Err(_) => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        String::from("获取每日大赛网地址失败"),
                    ));
                }
            }
        }
    }
}
