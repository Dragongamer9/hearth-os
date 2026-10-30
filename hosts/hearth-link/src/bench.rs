//! Hearth Benchmark over Hearth Link (hosts/hearth-tools, --app benchmark): your paired devices' latest benchmark
//! scores side by side, and the home-network test (how fast this device and a paired one move data over Hearth
//! Link's own connection, the one game copies use).
//!
//! Between devices: bench.summary (that device's summary.json, small), bench.down (n bytes back, at most 16 MiB),
//! bench.up (n bytes sent, answered with the count). From the app: bench.peers, bench.net {peer}.

use crate::node::Node;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_BLOB: usize = 16 << 20;
const PIECE: usize = 8 << 20;

/// This device's latest benchmark summary (written by Hearth Benchmark), or null. Never more than a few KB.
pub fn summary() -> Value {
    std::fs::read(hearth_core::paths::data_dir().join("benchmark/summary.json")).ok()
        .filter(|b| b.len() < 8192)
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .filter(Value::is_object)
        .unwrap_or(Value::Null)
}

/// `n` bytes of text to time the connection with.
pub fn blob(n: usize) -> String {
    let n = n.min(MAX_BLOB);
    let mut s = "0123456789abcdef".repeat(n / 16 + 1);
    s.truncate(n);
    s
}

/// Megabits per second for `bytes` in `secs`.
pub fn mbps(bytes: usize, secs: f64) -> f64 { if secs > 0.0 { ((bytes as f64 * 8.0 / secs / 1e6) * 10.0).round() / 10.0 } else { 0.0 } }

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() { return None; }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Some((v[v.len() / 2] * 10.0).round() / 10.0)
}

impl Node {
    /// Requests from paired devices.
    pub async fn bench_answer(self: Arc<Self>, _peer: &str, op: &str, req: &Value) -> Result<Value, String> {
        match op {
            "bench.summary" => Ok(json!({"summary": summary(), "name": self.name()})),
            "bench.down" => Ok(json!({"pad": blob(req["bytes"].as_u64().unwrap_or(0) as usize)})),
            "bench.up" => Ok(json!({"n": req["pad"].as_str().map(str::len).unwrap_or(0)})),
            _ => Err(format!("Unknown request: {op}")),
        }
    }

    /// From the app: bench.peers (every paired device's latest scores), bench.net {peer} (the home-network test).
    pub async fn bench_command(self: &Arc<Self>, cmd: &str, a: &Value) -> Result<Value, String> {
        match cmd {
            "bench.peers" => {
                let peers: Vec<(String, String)> = crate::lock(&self.peers).list.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
                let asks: Vec<_> = peers.into_iter().map(|(id, name)| {
                    let me = self.clone();
                    tokio::spawn(async move {
                        match tokio::time::timeout(Duration::from_secs(8), me.request(&id, &json!({"op": "bench.summary"}))).await {
                            Ok(Ok(v)) => json!({"id": id, "name": name, "online": true, "summary": v["summary"]}),
                            _ => json!({"id": id, "name": name, "online": false, "summary": Value::Null}),
                        }
                    })
                }).collect();
                let mut devices = Vec::new();
                for h in asks { if let Ok(v) = h.await { devices.push(v); } }
                Ok(json!({"devices": devices, "me": summary(), "name": self.name()}))
            }
            "bench.net" => {
                let id = a["peer"].as_str().unwrap_or("").to_string();
                let name = crate::lock(&self.peers).get(&id).map(|p| p.name.clone()).ok_or("That device isn't paired with this one")?;
                let ask = |req: Value| { let me = self.clone(); let id = id.clone(); async move {
                    tokio::time::timeout(Duration::from_secs(30), me.request(&id, &req)).await.map_err(|_| format!("{} didn't answer in time", id))?
                } };
                // The first request may have to connect: not counted.
                ask(json!({"op": "ping"})).await.map_err(|e| format!("Couldn't reach {name}: {e}"))?;
                let mut lat = Vec::new();
                for _ in 0..5 { let t = Instant::now(); ask(json!({"op": "ping"})).await?; lat.push(t.elapsed().as_secs_f64() * 1000.0); }
                let t = Instant::now();
                let mut down = 0;
                for _ in 0..4 { down += ask(json!({"op": "bench.down", "bytes": PIECE})).await?["pad"].as_str().map(str::len).unwrap_or(0); }
                let down_s = t.elapsed().as_secs_f64();
                let pad = blob(PIECE);
                let t = Instant::now();
                let mut up = 0;
                for _ in 0..4 { up += ask(json!({"op": "bench.up", "pad": pad})).await?["n"].as_u64().unwrap_or(0) as usize; }
                let up_s = t.elapsed().as_secs_f64();
                eprintln!("hearth-link: benchmark: {name}: {} Mbit/s down, {} up", mbps(down, down_s), mbps(up, up_s));
                Ok(json!({"name": name, "download": mbps(down, down_s), "upload": mbps(up, up_s), "latency": median(lat)}))
            }
            _ => Err(format!("Unknown command: {cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blobs_and_speeds() {
        assert_eq!(blob(5), "01234");
        assert_eq!(blob(usize::MAX).len(), MAX_BLOB);
        assert_eq!(mbps(1_000_000, 1.0), 8.0);
        assert_eq!(mbps(1, 0.0), 0.0);
        assert_eq!(median(vec![3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(vec![]), None);
    }
}
