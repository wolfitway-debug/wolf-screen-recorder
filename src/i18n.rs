use std::collections::HashMap;
use serde_json::Value;

#[allow(dead_code)]
pub struct I18nEngine {
    locales: HashMap<String, HashMap<String, String>>,
    active_lang: String,
}

impl I18nEngine {
    pub fn new() -> Self {
        let mut locales = HashMap::new();

        let languages = [
            ("en", include_str!("assets/locales/en.json")),
            ("ro", include_str!("assets/locales/ro.json")),
            ("es", include_str!("assets/locales/es.json")),
            ("de", include_str!("assets/locales/de.json")),
            ("fr", include_str!("assets/locales/fr.json")),
            ("ja", include_str!("assets/locales/ja.json")),
        ];

        for (lang, json_str) in languages {
            if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                if let Some(obj) = parsed.as_object() {
                    let mut map = HashMap::new();
                    for (k, v) in obj {
                        if let Some(s) = v.as_str() {
                            map.insert(k.clone(), s.to_string());
                        }
                    }
                    locales.insert(lang.to_string(), map);
                }
            }
        }

        Self {
            locales,
            active_lang: "en".to_string(),
        }
    }

    pub fn set_language(&mut self, lang: &str) {
        if self.locales.contains_key(lang) {
            self.active_lang = lang.to_string();
            println!("[I18nEngine] Active language set to: {}", lang);
        }
    }

    pub fn get_language(&self) -> &str {
        &self.active_lang
    }

    pub fn t(&self, key: &str) -> String {
        if let Some(dict) = self.locales.get(&self.active_lang) {
            if let Some(val) = dict.get(key) {
                return val.clone();
            }
        }
        // Fallback to English
        if let Some(en_dict) = self.locales.get("en") {
            if let Some(val) = en_dict.get(key) {
                return val.clone();
            }
        }
        key.to_string()
    }
}
