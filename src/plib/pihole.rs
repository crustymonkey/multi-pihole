#![allow(dead_code)]

use isahc::{prelude::*, Request, config::RedirectPolicy};
use serde_json::{self, Value, json};
use log::{error, warn, debug};
use std::collections::HashMap;
use super::config::PiServer;

pub struct Pihole {
    pub base_url: String,
    pub passwd: String,
    pub sid: Option<String>,  // This is the auth session ID that will be used
}

impl Pihole {
    pub fn new(base_url: &str, passwd: &str) -> Self {
        // Strip off any trailing slash
        let base = base_url.trim_matches('/');

        return Self {
            base_url: base.to_string(),
            passwd: passwd.to_string(),
            sid: None,
        };
    }

    /// Return a Pihole instance from a PiServer config object
    pub fn from_cfg(cfg: &PiServer) -> Self {
        return Self {
            base_url: cfg.base_url.clone(),
            passwd: cfg.passwd.clone(),
            sid: None,
        };
    }

    /// Authenticate with the server, this should mostly be just an internal
    /// call, but is still accessible in general
    pub fn auth(&mut self) -> Option<Value> {
        let data = json!({
            "password": self.passwd,
        });

        let res = self.run_post_cmd("auth", data);

        if let Some(r) = &res {
            if let Some(sess) = r.get("session") {
                // We didn't get an error here, so we can yank the session ID
                self.sid = Some(sess["sid"].as_str().unwrap().to_string());
            }
        }

        return res;
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

        return self.call_url(&url, None);
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

        return self.call_url(&url, None);
    }

    // Get the forward destinations
    pub fn get_fwd_dests(&self) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str("&getForwardDestinations");
        debug!("Calling url: {}", &url);

        return self.call_url(&url, None);
    }

    /// Get the query type stats from the server
    pub fn get_query_types(&self) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str("&getQueryTypes");
        debug!("Calling url: {}", &url);

        return self.call_url(&url, None);
    }

    /// Get all the DNS query data
    pub fn get_all_queries(&self) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str("&getAllQueries");
        debug!("Calling url: {}", &url);

        return self.call_url(&url, None);
    }

    /// Get a stats summary from the server
    pub fn summary(&self) -> Option<Value> {
        return self.run_get_cmd("stats/summary");
    }

    /// Get the status from the summary
    pub fn status(&self) -> Option<String> {
        return match self.summary() {
            None => None,
            Some(s) => Some(s.get("status").unwrap().to_string()),
        };
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
        return self.get_url_resp_body(&url, None);
    }

    fn run_get_cmd(&self, cmd: &str) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str(&format!("/{}", cmd));
        debug!("Calling url: {}", &url);

        return self.call_url(&url, None);
    }

    fn run_post_cmd(&self, cmd: &str, data: Value) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str(&format!("/{}", cmd));


        debug!("Calling url: {}", &url);
        return self.call_url(&url, Some(&data.to_string()));
    }

    fn call_url(&self, url: &str, data: Option<&str>) -> Option<Value> {
        let json_body = match self.get_url_resp_body(&url, data) {
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
        let json_body = match self.get_url_resp_body(&url, None) {
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

    fn get_url_resp_body(&self, url: &str, body: Option<&str>) -> Option<String> {
        // If we have a body, it's a POST request of type application/json
        let resp = match body {
            Some(b) => {
                let mut req = Request::post(url)
                    .redirect_policy(RedirectPolicy::Follow)
                    .header("Content-Type", "application/json");
                if let Some(sid) = &self.sid {
                    req = req.header("X-FTL-SID", sid);
                }
                req.body(b).unwrap().send()
            },
            None => {
                Request::get(url)
                    .redirect_policy(RedirectPolicy::Follow)
                    .header("X-FTL-SID", self.sid.as_ref().unwrap())
                    .body(()).unwrap()
                    .send()
            }
        };

        if let Err(e) = resp {
            error!("Failed to send request to {}: {}", url, e);
            return None;
        }

        let ret = match resp.unwrap().text() {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to get response body from {}: {}", url, e);
                return None;
            },
        };

        return Some(ret);
    }

    fn build_url(&self) -> String {
        let mut ret = "".to_string();

        ret.push_str(&self.base_url);
        // Add the api
        ret.push_str("/api");

        return ret;
    }
}