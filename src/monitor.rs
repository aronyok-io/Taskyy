use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use sysinfo::{Disks, Networks, ProcessRefreshKind, ProcessesToUpdate, System};
use crate::hardware::{self, Gpu};

pub struct Process {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub memory: u64,
    pub start_ticks: Option<u64>,
    pub parent: Option<u32>,
    pub status: String,
    pub thread_count: Option<usize>,
    pub run_time: u64,
    pub disk_read: u64,
    pub disk_written: u64,
}

pub struct Network {
    pub name: String,
    pub rx: f64,
    pub tx: f64,
    pub received: u64,
    pub sent: u64,
}

pub struct Storage {
    pub name: String,
    pub mount: String,
    pub filesystem: String,
    pub total: u64,
    pub used: u64,
    pub read: f64,
    pub write: f64,
}

pub struct Core {
    pub name: String,
    pub usage: f32,
    pub mhz: u64,
}

pub struct Snapshot {
    pub cpu: f32,
    pub used: u64,
    pub total: u64,
    pub processes: Vec<Process>,
    pub gpus: Vec<Gpu>,
    pub networks: Vec<Network>,
    pub disks: Vec<Storage>,
    pub cores: Vec<Core>,
    pub cpu_model: String,
    pub uptime: u64,
    pub swap_used: u64,
    pub swap_total: u64,
}

pub fn start() -> Arc<Mutex<Option<Snapshot>>> {
    let latest = Arc::new(Mutex::new(None));
    let output = Arc::downgrade(&latest);
    thread::spawn(move || {
        let mut system = System::new();
        let refresh = ProcessRefreshKind::nothing().with_cpu().with_memory().with_disk_usage();
        let mut networks = Networks::new_with_refreshed_list();
        let mut disks = Disks::new_with_refreshed_list();
        let mut last_sample = Instant::now();
        system.refresh_cpu_all();
        system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);

        loop {
            thread::sleep(Duration::from_secs(1));
            let Some(output) = output.upgrade() else { break };
            let now = Instant::now();
            let seconds = (now - last_sample).as_secs_f64();
            last_sample = now;
            networks.refresh(true);
            disks.refresh(true);
            system.refresh_cpu_all();
            system.refresh_memory();
            system.refresh_processes_specifics(ProcessesToUpdate::All, true, refresh);
            let total = system.total_memory();
            let mut interfaces: Vec<_> = networks.iter().map(|(name, counters)| Network {
                name: name.clone(),
                rx: counters.received() as f64 / seconds,
                tx: counters.transmitted() as f64 / seconds,
                received: counters.total_received(),
                sent: counters.total_transmitted(),
            }).collect();
            interfaces.sort_unstable_by(|a, b| a.name.cmp(&b.name));
            let snapshot = Snapshot {
                gpus: hardware::gpus(),
                networks: interfaces,
                disks: disks.list().iter().map(|disk| {
                    let usage = disk.usage();
                    Storage {
                        name: disk.name().to_string_lossy().into_owned(),
                        mount: disk.mount_point().display().to_string(),
                        filesystem: disk.file_system().to_string_lossy().into_owned(),
                        total: disk.total_space(),
                        used: disk.total_space().saturating_sub(disk.available_space()),
                        read: usage.read_bytes as f64 / seconds,
                        write: usage.written_bytes as f64 / seconds,
                    }
                }).collect(),
                cores: system.cpus().iter().map(|cpu| Core {
                    name: cpu.name().into(), usage: cpu.cpu_usage(), mhz: cpu.frequency(),
                }).collect(),
                cpu_model: system.cpus().first().map(|cpu| cpu.brand().into()).unwrap_or_default(),
                uptime: System::uptime(),
                swap_used: system.used_swap(),
                swap_total: system.total_swap(),
                cpu: system.global_cpu_usage(),
                used: total.saturating_sub(system.available_memory()),
                total,
                processes: system.processes().values().filter(|process| {
                    !matches!(process.thread_kind(), Some(sysinfo::ThreadKind::Userland))
                }).map(|process| Process {
                    pid: process.pid().as_u32(),
                    name: process.name().to_string_lossy().into_owned(),
                    cpu: process.cpu_usage(),
                    memory: process.memory(),
                    start_ticks: crate::process::start_ticks(process.pid().as_u32()).ok(),
                    parent: process.parent().map(|pid| pid.as_u32()),
                    status: process.status().to_string(),
                    thread_count: process.tasks().map(|tasks| tasks.len() + 1),
                    run_time: process.run_time(),
                    disk_read: process.disk_usage().total_read_bytes,
                    disk_written: process.disk_usage().total_written_bytes,
                }).collect(),
            };
            *output.lock().expect("snapshot lock") = Some(snapshot);
        }
    });
    latest
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;

    #[test]
    fn samples_live_linux_data() {
        let latest = start();
        let deadline = Instant::now() + Duration::from_secs(10);
        let snapshot = loop {
            if let Some(snapshot) = latest.lock().unwrap().take() {
                break snapshot;
            }
            assert!(Instant::now() < deadline, "no system snapshot received");
            thread::sleep(Duration::from_millis(50));
        };

        assert!(snapshot.total > 0);
        assert!(snapshot.used <= snapshot.total);
        assert!((0.0..=100.0).contains(&snapshot.cpu));
        assert!(snapshot.processes.iter().any(|entry| entry.pid == process::id()));
    }
}
