//! Application language only; document contents and diagnostic details stay intact.
use std::{
    collections::HashMap,
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering},
    },
};
static LANGUAGE: AtomicU8 = AtomicU8::new(0);
fn resolved(choice: &str) -> u8 {
    let system;
    let value = if choice == "system" {
        system = sys_locale::get_locale().unwrap_or_else(|| "en".into());
        &system
    } else {
        choice
    };
    u8::from(value.to_ascii_lowercase().starts_with("zh"))
}
pub fn initialize() {
    let choice = crate::preferences::language().unwrap_or_else(|| "system".into());
    set(&choice);
}
pub fn set(choice: &str) {
    LANGUAGE.store(resolved(choice), Ordering::Relaxed);
}
pub fn code() -> &'static str {
    if LANGUAGE.load(Ordering::Relaxed) == 1 {
        "zh-CN"
    } else {
        "en"
    }
}
pub fn tr(text: &str) -> String {
    static ZH: OnceLock<HashMap<String, String>> = OnceLock::new();
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    let map = if code() == "zh-CN" {
        ZH.get_or_init(|| {
            serde_json::from_str(include_str!("../../../assets/locales/zh-CN.json"))
                .expect("bundled Chinese catalog")
        })
    } else {
        EN.get_or_init(|| {
            serde_json::from_str(include_str!("../../../assets/locales/en.json"))
                .expect("bundled English catalog")
        })
    };
    map.get(text).cloned().unwrap_or_else(|| text.to_owned())
}

pub fn translations() -> serde_json::Value {
    serde_json::from_str(if code() == "zh-CN" {
        include_str!("../../../assets/locales/zh-CN.json")
    } else {
        include_str!("../../../assets/locales/en.json")
    })
    .expect("bundled locale catalog")
}
