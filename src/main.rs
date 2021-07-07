use std::{net::SocketAddr, path::PathBuf};

use chase::Chaser;
use chrono::{DateTime, NaiveDateTime, Utc};
use lazy_static::lazy_static;
use prometheus::{
    register_int_counter, register_int_counter_vec, register_int_gauge, IntCounter, IntCounterVec,
    IntGauge,
};
use regex::Regex;
use structopt::StructOpt;

lazy_static! {
    static ref RE_LOG: Regex =
        Regex::new(r#"^([0-9T\-:\.]+)\s?+([A-z_\.]+)\s?+([A-z_\.]+)\s?+:\s?+([A-Z]+)\s+(.*)$"#).unwrap();
    static ref RE_LOG_HARVEST: Regex = Regex::new(r#"^(\d+) plots were eligible for farming [0-9a-f]+\.\.\. Found (\d+) proofs\..*Total (\d+) plots"#).unwrap();

    pub static ref LOG_LINES: IntCounterVec =
        register_int_counter_vec!("chia_log_lines", "Number of total log lines parsed", &["level"]).unwrap();
    pub static ref HARVESTER_EVENTS_TOTAL: IntCounter =
        register_int_counter!("chia_harvester_events_total", "chia plot info").unwrap();
    pub static ref HARVESTER_PLOTS_ELIGIBLE: IntCounter =
        register_int_counter!("chia_harvester_plots_eligible", "chia plot info").unwrap();
    pub static ref HARVESTER_PLOTS_TOTAL: IntGauge =
        register_int_gauge!("chia_harvester_plots_total", "chia plot info").unwrap();
    pub static ref HARVESTER_PLOTS_PROOFS: IntCounter =
        register_int_counter!("chia_harvester_plots_proofs", "chia plot info").unwrap();

}

#[derive(Debug, Clone)]
struct LogEntry {
    pub date: DateTime<Utc>,
    pub app: String,
    pub module: String,
    pub level: String,
    pub text: String,
}

impl LogEntry {
    pub fn parse_str(log: &str) -> Option<Self> {
        let cap = RE_LOG.captures(log)?;

        Some(Self {
            date: DateTime::<Utc>::from_utc(
                NaiveDateTime::parse_from_str(&cap[1].to_string(), &"%Y-%m-%dT%H:%M:%S%.3f")
                    .ok()?,
                Utc,
            ),
            app: cap[2].to_string(),
            module: cap[3].to_string(),
            level: cap[4].to_string(),
            text: cap[5].to_string(),
        })
    }
}

async fn watch_harvester_task(log_file: PathBuf, listen_addr: SocketAddr) -> anyhow::Result<()> {
    tokio::spawn(watch_harvester_warp_server(listen_addr));

    println!("watching log file {}", log_file.to_str().unwrap());
    let real_now = Utc::now();

    let chaser = Chaser::new(&log_file);
    let (log_recv, _) = chaser.run_channel()?;

    while let Ok((log, _line, _pos)) = &mut log_recv.recv() {
        if let Some(entry) = LogEntry::parse_str(&log) {
            if entry.date < real_now {
                continue;
            }
            LOG_LINES.with_label_values(&[&entry.level]).inc();
            handle_log_entry(entry).await;
        }
    }

    Ok(())
}

async fn handle_log_entry(log: LogEntry) -> Option<()> {
    if &log.app == "harvester" {
        if let Some(harvest_stats) = RE_LOG_HARVEST.captures(&log.text) {
            HARVESTER_EVENTS_TOTAL.inc();

            let eligible: u64 = harvest_stats[1].parse().ok()?;
            let proofs: u64 = harvest_stats[2].parse().ok()?;
            let total_plots: i64 = harvest_stats[3].parse().ok()?;

            HARVESTER_PLOTS_ELIGIBLE.inc_by(eligible as u64);
            HARVESTER_PLOTS_PROOFS.inc_by(proofs);
            HARVESTER_PLOTS_TOTAL.set(total_plots);
        }
    }

    Some(())
}

async fn watch_harvester_warp_server(listen_addr: SocketAddr) {
    use prometheus::{Encoder, TextEncoder};
    use warp::{http, Filter};

    let metrics_route = warp::path!("metrics").and_then(|| async move {
        let mut buffer = Vec::new();
        let encoder = TextEncoder::new();

        let metric_families = prometheus::gather();
        encoder
            .encode(&metric_families, &mut buffer)
            .map_err(|_| warp::reject())?;

        match String::from_utf8(buffer.clone()).map_err(|_| warp::reject()) {
            Ok(output) => Ok(warp::reply::with_status(output, http::StatusCode::OK)),
            _ => Err(warp::reject()),
        }
    });

    println!("starting to listen on {:?}", listen_addr);
    warp::serve(metrics_route).run(listen_addr).await;
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "chia-harvester-watcher",
    about = "Watches your chia-blockchain harvester log and reports prometheus-ready metrics."
)]
struct Opt {
    #[structopt(long)]
    log_file: PathBuf,
    #[structopt(default_value = "[::]:4041", long)]
    listen_addr: SocketAddr,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    watch_harvester_task(opt.log_file, opt.listen_addr).await
}
