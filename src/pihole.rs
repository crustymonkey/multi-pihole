
use isahc::prelude::*;
use serde_json;
use log::{error, warn};
use std::collections::HashMap;

pub struct Pihole {
    base_url: String,
    api_key: String,
}

impl Pihole {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        // Strip off any trailing slash
        let base = base_url.trim_matches('/');

        return Self {
            base_url: base.to_string(),
            api_key: api_key.to_string()
        };
    }

    pub fn disable(&self, seconds: u64) -> bool {
        let mut url = self.build_url();

        url.push_str(&format!("&disable={}", seconds));
        let json_body = match self.get_url_resp_body(&url) {
            Some(b) => b,
            None => {
                return false;
            }
        };

        if let Ok(res) =
                serde_json::from_str::<HashMap<String, String>>(&json_body) {
            return res["status"] == "disabled";
        } else {
            warn!("Failed to deserialize response: {:?}", json_body);
            return false
        }
    }

    fn get_url_resp_body(&self, url: &str) -> Option<String> {
        let mut resp = match isahc::get(url) {
            Ok(r) => r,
            _ => {
                return None;
            },
        };

        let body = match resp.text() {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to get response body: {}", e);
                return None;
            },
        };

        return Some(body);
    }

    fn build_url(&self) -> String {
        let mut ret = "".to_string();

        ret.push_str(&self.base_url);
        // Add the api
        ret.push_str(&format!("/api.php?auth={}", &self.api_key));

        return ret;
    }
}