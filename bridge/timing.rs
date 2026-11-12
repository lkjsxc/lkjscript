//! Optional host observations. Timers and process resources do not affect acceptance.
use std::collections::BTreeMap;
use std::time::Instant;

pub struct Timings {
    enabled: bool,
    started: Instant,
    stage: &'static str,
    times: BTreeMap<&'static str, f64>,
    counts: BTreeMap<&'static str, usize>,
}
impl Timings {
    pub fn new() -> Self {
        Self {
            enabled: std::env::var("LKJSCRIPT_TIMINGS").as_deref() == Ok("1"),
            started: Instant::now(),
            stage: "startup",
            times: BTreeMap::new(),
            counts: BTreeMap::new(),
        }
    }
    pub fn measure<T>(&mut self, name: &'static str, work: impl FnOnce() -> T) -> T {
        self.stage = name;
        let start = Instant::now();
        let result = work();
        self.times
            .insert(name, start.elapsed().as_secs_f64() * 1000.0);
        result
    }
    pub fn count(&mut self, name: &'static str, count: usize) {
        self.counts.insert(name, count);
    }
    pub fn done(&mut self) {
        self.stage = "done";
    }
}
impl Drop for Timings {
    fn drop(&mut self) {
        if !self.enabled {
            return;
        }
        let mut fields = vec![format!(
            "\"route\":\"compiler\",\"stopped_stage\":\"{}\",\"total_ms\":{}",
            self.stage,
            self.started.elapsed().as_secs_f64() * 1000.0
        )];
        for (name, ms) in &self.times {
            fields.push(format!("\"{name}_ms\":{ms}"));
        }
        for (name, count) in &self.counts {
            fields.push(format!("\"{name}\":{count}"));
        }
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            if let Some(kib) = status
                .lines()
                .find(|line| line.starts_with("VmHWM:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|value| value.parse::<usize>().ok())
            {
                fields.push(format!("\"peak_rss_bytes\":{}", kib * 1024));
            }
        }
        eprintln!("LKJ_TIMING {{{}}}", fields.join(","));
    }
}
