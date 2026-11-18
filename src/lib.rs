//! Couch integration for Amazon Fire TV devices exposing the local Lightning
//! REST API on ports 8080 (HTTPS) and 8009 (DIAL wake).
//!
//! The device API uses a self-signed certificate. This client accepts that
//! certificate because the API is local-device-only; the host/IP is a user
//! supplied connection setting. The Fire TV client token is never stored in
//! those settings: protocol 3 pairing hands it to Couch as an opaque
//! `Credential`, which Couch persists in its private per-connection store.

use std::time::Duration;

use couch_sdk::{
    couch_model::commands::Function, Capability, ClientSettings, CodeAlphabet, Credential,
    DeviceClient, Error, PairFailure, PairFlow, PairInput, PairPrompt, PairStep, Result,
};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const API_KEY: &str = "0987654321";
const USER_AGENT: &str = "okhttp/4.10.0";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(4);
const DEFAULT_FRIENDLY_NAME: &str = "Couch Fire TV";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FireTvSettings {
    pub host: String,
    #[serde(default = "default_friendly_name")]
    pub friendly_name: String,
}

fn default_friendly_name() -> String {
    DEFAULT_FRIENDLY_NAME.to_owned()
}

impl ClientSettings for FireTvSettings {
    const FILE_PREFIX: &'static str = "firetv";

    fn validate(&self) -> Result<()> {
        let host = self.host.trim();
        let host_ok = !host.is_empty()
            && host.len() <= 253
            && host == self.host
            && host.bytes().all(|b| {
                b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':' | b'[' | b']')
            })
            && !host.starts_with('.')
            && !host.ends_with('.');
        let name_ok = !self.friendly_name.trim().is_empty()
            && self.friendly_name.len() <= 80
            && !self.friendly_name.chars().any(char::is_control);
        if host_ok && name_ok {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
}

/// A connected Fire TV integration. The API token is held only in memory;
/// the Couch host supplies it from its protected Credential store.
pub struct FireTvClient {
    tv: FireTvApi,
}

impl DeviceClient for FireTvClient {
    type Settings = FireTvSettings;

    const KIND: &'static str = "firetv";
    const LABEL: &'static str = "Amazon Fire TV";

    fn capabilities() -> &'static [Capability] {
        &[
            ("up", "Up"),
            ("down", "Down"),
            ("left", "Left"),
            ("right", "Right"),
            ("ok", "OK / select"),
            ("back", "Back"),
            ("home", "Home"),
            ("power-on", "Power on / wake"),
            ("play-pause", "Play / pause"),
            ("x:menu", "Menu"),
            ("x:fast-forward", "Fast forward"),
            ("x:rewind", "Rewind"),
        ]
    }

    /// A Fire TV connection is not usable until the PIN pairing flow has
    /// produced a token. The protocol-3 host calls `connect_with` after it has
    /// stored that token.
    fn connect(_settings: &FireTvSettings) -> Result<Self> {
        Err(Error::Unpaired)
    }

    fn connect_with(settings: &FireTvSettings, credential: Option<&Credential>) -> Result<Self> {
        settings.validate()?;
        let token = credential
            .and_then(|value| value.get().get("token"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or(Error::Unpaired)?;
        Ok(Self {
            tv: FireTvApi::new(&settings.host, Some(token.to_owned()))?,
        })
    }

    fn pair_start(
        settings: &FireTvSettings,
        _existing: Option<&Credential>,
    ) -> Result<Box<dyn PairFlow>> {
        settings.validate()?;
        Ok(Box::new(FireTvPairFlow {
            tv: FireTvApi::new(&settings.host, None)?,
            friendly_name: settings.friendly_name.clone(),
            started: false,
        }))
    }

    fn execute(&mut self, function: &Function) -> Result<()> {
        let function_id = function.id();
        match function_id.as_str() {
            "up" => self.tv.remote_key("dpad_up"),
            "down" => self.tv.remote_key("dpad_down"),
            "left" => self.tv.remote_key("dpad_left"),
            "right" => self.tv.remote_key("dpad_right"),
            "ok" => self.tv.remote_key("select"),
            "back" => self.tv.remote_key("back"),
            "home" => self.tv.remote_key("home"),
            "power-on" => self.tv.wake(),
            // The Fire TV Lightning API's `play` action is the remote's
            // play/pause control, as used by the community-documented API.
            "play-pause" => self.tv.media_action("play", None),
            "x:menu" => self.tv.remote_key("menu"),
            "x:fast-forward" => self.tv.scan("forward"),
            "x:rewind" => self.tv.scan("back"),
            _ => Err(Error::Unsupported),
        }
    }
}

/// The Fire TV's four-digit PIN is collected by Couch, not by this process.
/// The resulting token is returned as a Credential so it never passes through
/// a normal settings field or a log line.
struct FireTvPairFlow {
    tv: FireTvApi,
    friendly_name: String,
    started: bool,
}

impl PairFlow for FireTvPairFlow {
    fn step(&mut self, input: Option<PairInput>) -> Result<PairStep> {
        if !self.started {
            if input.is_some() {
                return Err(Error::Protocol);
            }
            self.tv.pair_start(&self.friendly_name)?;
            self.started = true;
            return Ok(PairStep::waiting(
                PairPrompt::enter_code(4, CodeAlphabet::Digits)
                    .saying("Enter the four-digit PIN shown on your Fire TV"),
                0,
            ));
        }

        let Some(PairInput::Code { code }) = input else {
            return Err(Error::Protocol);
        };
        match self.tv.pair_finish(&code) {
            Ok(token) => {
                let credential = Credential::new(json!({ "token": token }))?;
                Ok(PairStep::done(credential, "Paired with Amazon Fire TV"))
            }
            Err(Error::Rejected) => {
                Ok(PairStep::failed(PairFailure::WrongCode)
                    .because("The Fire TV rejected that PIN"))
            }
            Err(Error::Timeout) => Ok(PairStep::failed(PairFailure::TimedOut)
                .because("The Fire TV pairing request timed out")),
            Err(Error::Transport) => {
                Ok(PairStep::failed(PairFailure::Unreachable)
                    .because("Could not reach the Fire TV"))
            }
            Err(_) => Ok(PairStep::failed(PairFailure::Refused)
                .because("The Fire TV could not complete pairing")),
        }
    }
}

struct FireTvApi {
    host: String,
    token: Option<String>,
    client: Client,
}

#[derive(Deserialize)]
struct PairResponse {
    #[serde(default)]
    token: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

impl FireTvApi {
    fn new(host: &str, token: Option<String>) -> Result<Self> {
        let client = Client::builder()
            // Fire TV's local Lightning endpoint uses a self-signed cert.
            // This is intentionally scoped to this device API client.
            .danger_accept_invalid_certs(true)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(map_http_error)?;
        Ok(Self {
            host: host.to_owned(),
            token,
            client,
        })
    }

    fn authority(&self) -> String {
        if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        }
    }

    fn https_url(&self, path: &str) -> String {
        format!("https://{}:8080{}", self.authority(), path)
    }

    fn common_headers(&self, request: RequestBuilder, include_token: bool) -> RequestBuilder {
        let request = request
            .header("X-Api-Key", API_KEY)
            .header("User-Agent", USER_AGENT)
            .header("Content-Type", "application/json; charset=utf-8");
        if include_token {
            if let Some(token) = self.token.as_deref() {
                return request.header("X-Client-Token", token);
            }
        }
        request
    }

    fn send_api(&self, path: &str, body: Option<Value>, include_token: bool) -> Result<()> {
        let mut request = self.client.post(self.https_url(path));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = self
            .common_headers(request, include_token)
            .send()
            .map_err(map_http_error)?;
        check_status(response.status())
    }

    fn pair_start(&self, friendly_name: &str) -> Result<()> {
        self.send_api(
            "/v1/FireTV/pin/display",
            Some(json!({ "friendlyName": friendly_name })),
            false,
        )
    }

    fn pair_finish(&self, pin: &str) -> Result<String> {
        let request = self
            .client
            .post(self.https_url("/v1/FireTV/pin/verify"))
            .json(&json!({ "pin": pin }));
        let response = self
            .common_headers(request, false)
            .send()
            .map_err(map_http_error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(if status.is_client_error() {
                Error::Rejected
            } else {
                Error::Transport
            });
        }
        let parsed: PairResponse = response.json().map_err(|_| Error::Protocol)?;
        parsed
            .token
            .or(parsed.description)
            .filter(|value| !value.is_empty())
            .ok_or(Error::Protocol)
    }

    fn remote_key(&self, action: &str) -> Result<()> {
        self.send_api(&format!("/v1/FireTV?action={action}"), None, true)
    }

    fn media_action(&self, action: &str, body: Option<Value>) -> Result<()> {
        self.send_api(&format!("/v1/media?action={action}"), body, true)
    }

    fn scan(&self, direction: &str) -> Result<()> {
        let down = json!({
            "direction": direction,
            "keyAction": { "keyActionType": "keyDown" }
        });
        let up = json!({
            "direction": direction,
            "keyAction": { "keyActionType": "keyUp" }
        });
        self.media_action("scan", Some(down))?;
        // Always release after a successful keyDown. A failed keyUp is
        // surfaced; commands are never retried because delivery is ambiguous.
        self.media_action("scan", Some(up))
    }

    fn wake(&self) -> Result<()> {
        let url = format!("http://{}:8009/apps/FireTVRemote", self.authority());
        let response = self.client.post(url).send().map_err(map_http_error)?;
        check_status(response.status())
    }
}

fn map_http_error(error: reqwest::Error) -> Error {
    if error.is_timeout() {
        Error::Timeout
    } else {
        Error::Transport
    }
}

fn check_status(status: StatusCode) -> Result<()> {
    if status.is_success() {
        Ok(())
    } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        Err(Error::Unpaired)
    } else {
        Err(Error::Rejected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_reject_url_injection_and_control_characters() {
        let valid = FireTvSettings {
            host: "10.0.0.21".into(),
            friendly_name: "Living Room Fire TV".into(),
        };
        assert!(valid.validate().is_ok());

        for host in ["", " https://10.0.0.21", "10.0.0.21/path", "10.0.0.21\n"] {
            let invalid = FireTvSettings {
                host: host.into(),
                friendly_name: DEFAULT_FRIENDLY_NAME.into(),
            };
            assert!(invalid.validate().is_err(), "accepted host {host:?}");
        }
    }

    #[test]
    fn ipv6_authority_is_bracketed_for_urls() {
        let api = FireTvApi::new("fe80::1", None).unwrap();
        assert_eq!(api.authority(), "[fe80::1]");
    }

    #[test]
    fn token_is_not_in_settings_or_debuggable_pair_flow_state() {
        let token = "do-not-log-this";
        let credential = Credential::new(json!({ "token": token })).unwrap();
        assert!(!format!("{credential:?}").contains(token));
        assert!(credential.get().get("token").unwrap().as_str().is_some());
    }
}
