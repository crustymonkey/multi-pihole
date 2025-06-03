#![allow(dead_code)]

use isahc::{prelude::*, Request, config::RedirectPolicy};
use serde_json::{self, Value, json};
use log::{error, warn, debug};
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

    /// Return the version of this server
    pub fn version(&self) -> Option<Value> {
        return self.run_get_cmd("info/version");
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
        url.push_str(&format!("/stats/top_domains?count={}", _top_n));
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
        url.push_str(&format!("/stats/top_clients?count={}", _top_n));
        debug!("Calling url: {}", &url);

        return self.call_url(&url, None);
    }

    // Get the forward destinations
    pub fn get_upstreams(&self) -> Option<Value> {
        return self.run_get_cmd("stats/upstreams");
    }

    /// Get the query type stats from the server
    pub fn get_query_types(&self) -> Option<Value> {
        return self.run_get_cmd("stats/query_types");
    }

    /// TODO: FIX Get all the DNS query data
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
        let res = self.run_get_cmd("dns/blocking");
        return match res {
            None => None,
            Some(s) => {
                Some(s["blocking"].to_string())
            },
        };
    }

    /// Enable a server
    pub fn enable(&self) -> Option<Value> {
        let body = json!({
            "blocking": true,
            "timer": null
        });

        return self.run_post_cmd("dns/blocking", body);
    }

    /// Disable a server for a specified number of seconds
    pub fn disable(&self, seconds: usize) -> Option<Value> {
        let body = json!({
            "blocking": false,
            "timer": seconds
        });

        return self.run_post_cmd("dns/blocking", body);
    }

    /// Get the most recently blocked domain
    pub fn recent_blocked(&self) -> Option<Value> {
        return self.run_get_cmd("stats/recent_blocked");
    }

    /*
     * Private methods for internal use
     */

    /// This is a high level function to run a GET command with no frills
    fn run_get_cmd(&self, cmd: &str) -> Option<Value> {
        let mut url = self.build_url();
        url.push_str(&format!("/{}", cmd));
        debug!("Calling url: {}", &url);

        return self.call_url(&url, None);
    }

    /// This is a high level function to run a POST command with no frills
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pihole_new() {
        let pihole = Pihole::new("http://localhost", "password");
        assert_eq!(pihole.base_url, "http://localhost");
        assert_eq!(pihole.passwd, "password");
        assert!(pihole.sid.is_none());
    }

    #[test]
    fn test_pihole_from_cfg() {
        let cfg = PiServer {
            base_url: "http://localhost".to_string(),
            passwd: "password".to_string(),
        };
        let pihole = Pihole::from_cfg(&cfg);
        assert_eq!(pihole.base_url, "http://localhost");
        assert_eq!(pihole.passwd, "password");
        assert!(pihole.sid.is_none());
    }

    #[test]
    fn test_build_url() {
        let pihole = Pihole::new("http://localhost", "password");
        let url = pihole.build_url();
        assert_eq!(url, "http://localhost/api");
    }
}