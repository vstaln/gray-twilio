//! twilio — SMS channel: twilio_send posts a message; twilio_inbox polls
//! recent inbound messages (Twilio REST has no push — polling keeps this
//! self-contained without a public webhook).
//! Config: ~/.gray/twilio/config.json {"sid","token","from"} or env
//! TWILIO_SID / TWILIO_TOKEN / TWILIO_FROM.

use std::io::{BufRead, Write};
use serde_json::{json, Value};

const NAME: &str = "twilio";

fn manifest() -> Value {
    json!({
        "name": NAME, "display_name": "Twilio SMS",
        "version": env!("CARGO_PKG_VERSION"), "protocol": "1.1",
        "tools": [
            {"name": "twilio_send",
             "description": "Send an SMS via Twilio.",
             "parameters": {"type": "object", "properties": {
                "to": {"type": "string", "description": "E.164 number"},
                "body": {"type": "string"}}, "required": ["to","body"]}},
            {"name": "twilio_inbox",
             "description": "Recent inbound SMS to the configured number (polled — no webhook needed).",
             "parameters": {"type": "object", "properties": {
                "limit": {"type": "integer"}}}},
        ],
        "commands": ["/twilio"], "subcommands": ["twilio"],
    })
}

fn config_path() -> std::path::PathBuf {
    let home = std::env::var_os("GRAY_HOME").map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".gray")))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join("twilio").join("config.json")
}
fn read_config() -> Value {
    std::fs::read_to_string(config_path()).ok()
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_else(|| json!({}))
}
fn cfg(k: &str, env: &str) -> Option<String> {
    std::env::var(env).ok().filter(|s| !s.is_empty())
        .or_else(|| read_config()[k].as_str().map(str::to_string))
}
fn api_base() -> String {
    std::env::var("TWILIO_API")
        .unwrap_or_else(|_| "https://api.twilio.com".into())
}

fn creds() -> Result<(String, String, String), String> {
    Ok((
        cfg("sid", "TWILIO_SID").ok_or("no sid — `twilio setup --sid AC… --token … --from +1…`")?,
        cfg("token", "TWILIO_TOKEN").ok_or("no token")?,
        cfg("from", "TWILIO_FROM").ok_or("no from number")?,
    ))
}

fn b64(s: &str) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::new();
    for c in s.as_bytes().chunks(3) {
        let n = match c.len() {
            3 => ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | c[2] as u32,
            2 => ((c[0] as u32) << 16) | ((c[1] as u32) << 8),
            _ => (c[0] as u32) << 16,
        };
        o.push(T[((n >> 18) & 63) as usize] as char);
        o.push(T[((n >> 12) & 63) as usize] as char);
        o.push(if c.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        o.push(if c.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    o
}

fn send(a: &Value) -> Result<String, String> {
    let (sid, token, from) = creds()?;
    let url = format!("{}/2010-04-01/Accounts/{sid}/Messages.json", api_base());
    let body = format!("To={}&From={}&Body={}",
        urlenc(a["to"].as_str().ok_or("to required")?),
        urlenc(&from), urlenc(a["body"].as_str().ok_or("body required")?));
    match ureq::post(&url)
        .set("Authorization", &format!("Basic {}", b64(&format!("{sid}:{token}"))))
        .set("Content-Type", "application/x-www-form-urlencoded")
        .timeout(std::time::Duration::from_secs(20))
        .send_string(&body) {
        Ok(r) => {
            let v: Value = r.into_json().map_err(|e| e.to_string())?;
            Ok(format!("sent sid {}", v["sid"].as_str().unwrap_or("?")))
        }
        Err(ureq::Error::Status(c, r)) => Err(format!("HTTP {c}: {}",
            r.into_string().unwrap_or_default().chars().take(300).collect::<String>())),
        Err(e) => Err(e.to_string()),
    }
}

fn inbox(a: &Value) -> Result<String, String> {
    let (sid, token, from) = creds()?;
    let n = a["limit"].as_u64().unwrap_or(15).min(50);
    let url = format!(
        "{}/2010-04-01/Accounts/{sid}/Messages.json?To={}&PageSize={n}", api_base(),
        urlenc(&from));
    match ureq::get(&url)
        .set("Authorization", &format!("Basic {}", b64(&format!("{sid}:{token}"))))
        .timeout(std::time::Duration::from_secs(20)).call() {
        Ok(r) => {
            let v: Value = r.into_json().map_err(|e| e.to_string())?;
            let msgs = v["messages"].as_array().cloned().unwrap_or_default();
            if msgs.is_empty() { return Ok("no inbound messages".into()); }
            Ok(msgs.iter().map(|m| format!("{} ← {}: {}",
                m["date_sent"].as_str().unwrap_or(""),
                m["from"].as_str().unwrap_or("?"),
                m["body"].as_str().unwrap_or(""))).collect::<Vec<_>>().join("\n"))
        }
        Err(ureq::Error::Status(c, r)) => Err(format!("HTTP {c}: {}",
            r.into_string().unwrap_or_default().chars().take(300).collect::<String>())),
        Err(e) => Err(e.to_string()),
    }
}

fn urlenc(s: &str) -> String {
    s.bytes().map(|b| match b {
        b'A'..=b'Z'|b'a'..=b'z'|b'0'..=b'9'|b'-'|b'_'|b'.'|b'~' => (b as char).to_string(),
        _ => format!("%{b:02X}"),
    }).collect()
}

fn tool(name: &str, args: &Value) -> Value {
    let r = match name {
        "twilio_send" => send(args),
        "twilio_inbox" => inbox(args),
        t => Err(format!("unknown tool {t}")),
    };
    match r {
        Ok(c) => json!({"result": {"content": c}}),
        Err(e) => json!({"result": {"content": e, "is_error": true}}),
    }
}

fn cli(argv: &[String]) -> Result<String, String> {
    match argv.first().map(String::as_str) {
        Some("setup") => {
            let mut c = read_config();
            let mut it = argv[1..].iter();
            while let Some(f) = it.next() {
                match f.as_str() {
                    "--sid" => c["sid"] = json!(it.next().ok_or("--sid needs value")?),
                    "--token" => c["token"] = json!(it.next().ok_or("--token needs value")?),
                    "--from" => c["from"] = json!(it.next().ok_or("--from needs number")?),
                    o => return Err(format!("unknown flag {o}")),
                }
            }
            std::fs::create_dir_all(config_path().parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(config_path(), serde_json::to_string_pretty(&c).unwrap())
                .map_err(|e| e.to_string())?;
            #[cfg(unix)] {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(config_path(),
                    std::fs::Permissions::from_mode(0o600));
            }
            Ok("saved".into())
        }
        Some("status") | None => Ok(format!("Twilio\n  creds: {}",
            if creds().is_ok() { "set" } else { "MISSING" })),
        Some("-h" | "--help" | "help") => Ok("usage: twilio setup | status".into()),
        Some(o) => Err(format!("unknown verb {o}")),
    }
}

fn sidecar() -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        let Ok(v) = serde_json::from_str::<Value>(&line) else { continue };
        let method = v["method"].as_str().unwrap_or("");
        match v.get("id").cloned() {
            None if method == "plugin/shutdown" => return Ok(()),
            None => {}
            Some(id) => {
                let reply = if method == "plugin/manifest" {
                    json!({"id": id, "result": manifest()})
                } else if method == "tool/call" {
                    let mut r = tool(v["params"]["name"].as_str().unwrap_or(""), &v["params"]["args"]);
                    r["id"] = id; r
                } else if method == "command/run" {
                    let argv: Vec<String> = v["params"]["argv"].as_array()
                        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
                        .unwrap_or_default();
                    let text = cli(&argv).unwrap_or_else(|e| e);
                    json!({"id": id, "result": {"text": text}})
                } else {
                    json!({"id": id, "error": {"code": -32601,
                        "message": format!("method not found: {method}")}})
                };
                writeln!(out, "{reply}")?; out.flush()?;
            }
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<String> = match args.first().map(String::as_str) {
        Some("twilio") => args[1..].to_vec(), _ => args };
    match args.first().map(String::as_str) {
        Some("manifest") => println!("{}", manifest()),
        Some("sidecar") => { let _ = sidecar(); }
        _ => {
            if args.is_empty() {
                if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                    println!("twilio — `twilio setup --sid AC… --token … --from +1…`");
                    return;
                }
                let _ = sidecar(); return;
            }
            match cli(&args) {
                Ok(t) => println!("{t}"),
                Err(e) => { eprintln!("{e}"); std::process::exit(1); }
            }
        }
    }
}
