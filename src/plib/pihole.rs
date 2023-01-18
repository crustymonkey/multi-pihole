#![allow(dead_code)]

use isahc::{prelude::*, Request, config::RedirectPolicy};
use serde_json::{self, Value};
use log::{error, warn, debug};
use std::collections::HashMap;
use super::config::PiServer;

pub struct Pihole {
    pub base_url: String,
    pub api_key: String,
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

    /// Return a Pihole instance from a PiServer config object
    pub fn from_cfg(cfg: &PiServer) -> Self {
        return Self {
            base_url: cfg.base_url.clone(),
            api_key: cfg.api_key.clone(),
        };
    }

    /// Get the type from the server, FTL or PHP
    pub fn stype(&self) -> Option<Value> {
        return self.run_get_cmd("type");
    }

    /// Return the version of this server
    pub fn version(&self) -> Option<Value> {
        return self.run_get_cmd("version");
    }

    /// Return today's data in 10 minute intervals
    pub fn over_time_data_10_mins(&self) -> Option<Value> {
        return self.run_get_cmd("overTimeData10mins");
    }

    /// Get the top domain and top advertisers lists
    pub fn top_items(&self, top_n: Option<usize>) -> Option<Value> {
        let _top_n = match top_n {
            None => 25,
            Some(n) => n,
        };

        let mut url = self.build_url();
        url.push_str(&format!("&topItems={}", _top_n));
        debug!("Calling url: {}", &url);

        return self.call_url(&url);
    }

    /// Get the top clients
    pub fn top_clients(&self, top_n: Option<usize>) -> Option<Value> {
        let _top_n = match top_n {
            None => 25,
            Some(n) => n,
        };

        let mut url = self.build_url();
        url.push_str(&format!("&topClients={}", _top_n));
        debug!("Calling url: {}", &url);

        return self.call_url(&url);
    }

    // Get the forward destinations
    pub fn get_fwd_dests(&self) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str("&getForwardDestinations");
        debug!("Calling url: {}", &url);

        return self.call_url(&url);
    }

    /// Get the query type stats from the server
    pub fn get_query_types(&self) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str("&getQueryTypes");
        debug!("Calling url: {}", &url);

        return self.call_url(&url);
    }

    /// Get all the DNS query data
    pub fn get_all_queries(&self) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str("&getAllQueries");
        debug!("Calling url: {}", &url);

        return self.call_url(&url);
    }

    /// Get a stats summary from the server
    pub fn summary(&self) -> Option<Value> {
        return self.run_get_cmd("summaryRaw");
    }

    /// Enable a server
    pub fn enable(&self) -> bool {
        let mut url = self.build_url();
        url.push_str("&enable");
        debug!("Calling url: {}", &url);

        return self.enable_disable(&url, "enabled");
    }

    /// Disable a server for a specified number of seconds
    pub fn disable(&self, seconds: usize) -> bool {
        let mut url = self.build_url();
        url.push_str(&format!("&disable={}", seconds));
        debug!("Calling url: {}", &url);

        return self.enable_disable(&url, "disabled");
    }

    /// Get the most recently blocked domain
    pub fn recent_blocked(&self) -> Option<String> {
        let mut url = self.build_url();
        url.push_str(&format!("&{}", "recentBlocked"));
        debug!("Calling url: {}", &url);
        return self.get_url_resp_body(&url);
    }

    fn run_get_cmd(&self, cmd: &str) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str(&format!("&{}", cmd));
        debug!("Calling url: {}", &url);

        return self.call_url(&url);
    }

    fn call_url(&self, url: &str) -> Option<Value> {
        let json_body = match self.get_url_resp_body(&url) {
            Some(b) => b,
            _ => { return None; }
        };

        debug!("Received response from server: {}", &json_body);

        match serde_json::from_str::<Value>(&json_body) {
            Ok(res) => {
                return Some(res);
            },
            Err(e) => {
                warn!("Failed to parse JSON body {}\n{}", e, &json_body);
                return None
            }
        }
    }

    fn enable_disable(&self, url: &str, expect: &str) -> bool {
        let json_body = match self.get_url_resp_body(&url) {
            Some(b) => b,
            None => {
                return false;
            }
        };

        debug!("Received response from server: {}", &json_body);

        match serde_json::from_str::<HashMap<String, String>>(&json_body) {
            Ok(res) => {   
                return res["status"] == expect;
            },
            Err(e) => {
                warn!("Failed to deserialize response from {}: {}", url, e);
                return false;
            },
        }
    }

    fn get_url_resp_body(&self, url: &str) -> Option<String> {
        let mut resp = match 
            Request::get(url)
                .redirect_policy(RedirectPolicy::Follow)
                .body(()).unwrap()
                .send()
        {
            Ok(r) => r,
            _ => {
                return None;
            },
        };


        let body = match resp.text() {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to get response body from {}: {}", url, e);
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