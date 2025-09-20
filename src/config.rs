use config::{Config, ConfigError};
use serde::Deserialize;
use std::sync::LazyLock;

pub static APP_CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    let config = load_config().unwrap();
    eprintln!("加载配置成功：{:#?}", config);
    config
});

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub config: InnerConfig,
}

#[derive(Debug, Deserialize)]
pub struct InnerConfig {
    pub heiliao: String,
    pub meiridasai: String,
    pub caoliu: String,
}

pub fn load_config() -> Result<AppConfig, ConfigError> {
    let settings = Config::builder()
        .add_source(config::File::with_name("configs"))
        .build()
        .unwrap();
    Ok(settings.try_deserialize::<AppConfig>().unwrap())
}
