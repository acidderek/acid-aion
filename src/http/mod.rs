use std::sync::{Arc, Mutex};

use tiny_http::{Header, Response, Server};

use crate::kernel::{compute_overall_health, TelemetrySnapshot};
use crate::memory::MemoryBus;
use crate::organism::{self, SystemTopology};

mod homepage;

pub struct HttpServer {
    addr: String,
}

impl HttpServer {
    pub fn new(addr: &str) -> Self {
        Self {
            addr: addr.to_string(),
        }
    }

    pub fn start(
        &self,
        topology: Arc<Mutex<SystemTopology>>,
        metrics: Arc<Mutex<Option<TelemetrySnapshot>>>,
        memory: MemoryBus,
    ) {
        let addr = self.addr.clone();

        std::thread::spawn(move || {
            let server = Server::http(&addr).unwrap();
            println!("[AION-HTTP] Listening on http://{}", addr);

            for req in server.incoming_requests() {
                let url = req.url().to_string();

                // Snapshot of health + awareness for each request.
                let (health_score, health_label, awareness_score, awareness_label) = {
                    if let Ok(topo) = topology.lock() {
                        let h = compute_overall_health(&*topo);
                        let hl = if h >= 0.85 {
                            "ok"
                        } else if h >= 0.60 {
                            "degraded"
                        } else if h >= 0.35 {
                            "impaired"
                        } else if h > 0.0 {
                            "critical"
                        } else {
                            "failed"
                        };

                        let a = organism::compute_awareness(&*topo);
                        let al = organism::describe_awareness(a);

                        (h, hl.to_string(), a, al.to_string())
                    } else {
                        (1.0, "ok".to_string(), 1.0, "optimal".to_string())
                    }
                };

                let response = match url.as_str() {
                    "/" => {
                        let html = homepage::homepage_html(
                            health_score,
                            &health_label,
                            awareness_score,
                            &awareness_label,
                        );

                        Response::from_string(html).with_header(
                            Header::from_bytes("Content-Type", "text/html; charset=utf-8")
                                .unwrap(),
                        )
                    }

                    "/status" => {
                        let json = format!(
                            r#"{{"health":{{"score":{:.3},"label":"{}"}},"awareness":{{"score":{:.3},"label":"{}"}}}}"#,
                            health_score, health_label, awareness_score, awareness_label
                        );

                        Response::from_string(json).with_header(
                            Header::from_bytes("Content-Type", "application/json").unwrap(),
                        )
                    }

                    "/metrics" => {
                        let guard = metrics.lock().unwrap();

                        if let Some(snap) = *guard {
                            let body = format!(
                                concat!(
                                    r#"{{"cpu":{{"cpu_load":{:.3},"cpu_temp_c":{:.1},"throttling_events":{},"gpu_load":{:.3},"gpu_mem_util":{:.3}}},"#,
                                    r#""memory":{{"ram_used_ratio":{:.3},"swap_used_ratio":{:.3},"major_page_faults":{:.3},"disk_latency_ms":{:.3}}},"#,
                                    r#""io":{{"net_packet_loss":{:.3},"net_latency_ms":{:.3},"io_queue_depth":{:.3},"io_error_rate":{:.3}}}}}"#
                                ),
                                snap.cpu.cpu_load,
                                snap.cpu.cpu_temp_c,
                                snap.cpu.throttling_events,
                                snap.cpu.gpu_load,
                                snap.cpu.gpu_mem_util,
                                snap.mem.ram_used_ratio,
                                snap.mem.swap_used_ratio,
                                snap.mem.major_page_faults,
                                snap.mem.disk_latency_ms,
                                snap.io.net_packet_loss,
                                snap.io.net_latency_ms,
                                snap.io.io_queue_depth,
                                snap.io.io_error_rate,
                            );

                            Response::from_string(body).with_header(
                                Header::from_bytes("Content-Type", "application/json").unwrap(),
                            )
                        } else {
                            Response::from_string(r#"{"error":"metrics not yet available"}"#)
                                .with_status_code(503)
                                .with_header(
                                    Header::from_bytes(
                                        "Content-Type",
                                        "application/json",
                                    )
                                    .unwrap(),
                                )
                        }
                    }

                    "/mem" => {
                        // Text dump of shared working memory (global + others).
                        let dump = memory.dump();
                        Response::from_string(dump).with_header(
                            Header::from_bytes(
                                "Content-Type",
                                "text/plain; charset=utf-8",
                            )
                            .unwrap(),
                        )
                    }

                    _ => {
                        Response::from_string(r#"{"error":"not found"}"#)
                            .with_status_code(404)
                            .with_header(
                                Header::from_bytes("Content-Type", "application/json").unwrap(),
                            )
                    }
                };

                let _ = req.respond(response);
            }
        });
    }
}
