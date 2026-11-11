use std::{fs, path::Path, process::Command};

pub struct Gpu {
    pub name: String,
    pub usage: Option<f32>,
    pub used_memory: Option<u64>,
    pub total_memory: Option<u64>,
    pub temperature: Option<f32>,
    pub clock: Option<f32>,
    pub watts: Option<f32>,
    pub driver: String,
}

fn read<T: std::str::FromStr>(path: &Path) -> Option<T> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

pub fn gpus() -> Vec<Gpu> {
    let mut gpus = nvidia();
    let queried_nvidia = !gpus.is_empty();
    let Ok(cards) = fs::read_dir("/sys/class/drm") else { return gpus };
    let mut cards: Vec<_> = cards.flatten().collect();
    cards.sort_unstable_by_key(|card| card.file_name());

    for card in cards {
        let name = card.file_name();
        let Some(index) = name.to_str().and_then(|name| name.strip_prefix("card")) else { continue };
        if index.parse::<u32>().is_err() { continue; }
        let card_path = card.path();
        let device = card_path.join("device");
        let Ok(vendor) = fs::read_to_string(device.join("vendor")) else { continue };
        let vendor = match vendor.trim() {
            "0x10de" if queried_nvidia => continue,
            "0x10de" => "NVIDIA",
            "0x1002" => "AMD",
            "0x8086" => "Intel",
            _ => "DRM",
        };
        let model = fs::read_to_string(device.join("device")).unwrap_or_default();
        let driver = fs::read_link(device.join("driver")).ok()
            .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "unknown driver".into());
        let sensors = fs::read_dir(device.join("hwmon")).ok()
            .and_then(|mut entries| entries.find_map(Result::ok)).map(|entry| entry.path());
        gpus.push(Gpu {
            name: format!("{vendor} {} · card{index}", model.trim()),
            usage: read::<f32>(&device.join("gpu_busy_percent")).filter(|value| (0.0..=100.0).contains(value)),
            used_memory: read(&device.join("mem_info_vram_used")),
            total_memory: read(&device.join("mem_info_vram_total")),
            temperature: sensors.as_ref().and_then(|path| read::<f32>(&path.join("temp1_input"))).map(|value| value / 1000.0),
            watts: sensors.as_ref().and_then(|path| read::<f32>(&path.join("power1_average"))).map(|value| value / 1_000_000.0),
            clock: [
                card_path.join("gt/gt0/rps_act_freq_mhz"),
                card_path.join("gt_act_freq_mhz"),
                card_path.join("gt_cur_freq_mhz"),
                device.join("gt_cur_freq_mhz"),
            ].iter().find_map(|path| read::<f32>(path).filter(|value| value.is_finite() && *value >= 0.0)),
            driver,
        });
    }
    gpus
}

fn nvidia() -> Vec<Gpu> {
    if !Path::new("/usr/bin/nvidia-smi").is_file() {
        return Vec::new();
    }
    let Ok(output) = Command::new("/usr/bin/timeout")
        .args(["-k", "0.2s", "0.7s", "/usr/bin/nvidia-smi",
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu,clocks.current.graphics,power.draw,driver_version",
            "--format=csv,noheader,nounits"])
        .output() else { return Vec::new() };
    if !output.status.success() { return Vec::new(); }
    String::from_utf8_lossy(&output.stdout).lines().filter_map(parse_nvidia).collect()
}

fn parse_nvidia(line: &str) -> Option<Gpu> {
    let columns: Vec<_> = line.rsplitn(8, ',').map(str::trim).collect();
    let [driver, watts, clock, temperature, total, used, usage, name] = columns.as_slice() else { return None };
    Some(Gpu {
        name: name.to_string(),
        usage: usage.parse::<f32>().ok().filter(|value| (0.0..=100.0).contains(value)),
        used_memory: used.parse::<u64>().ok().and_then(|value| value.checked_mul(1 << 20)),
        total_memory: total.parse::<u64>().ok().and_then(|value| value.checked_mul(1 << 20)),
        temperature: temperature.parse().ok(),
        clock: clock.parse().ok(),
        watts: watts.parse().ok(),
        driver: format!("NVIDIA {driver}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_nvidia_samples_and_missing_metrics() {
        let gpu = parse_nvidia("GeForce Test, 37, 1024, 8192, 50, 900, [N/A], 590.01").unwrap();
        assert_eq!(gpu.usage, Some(37.0));
        assert_eq!(gpu.used_memory, Some(1 << 30));
        assert!(gpu.watts.is_none());
        assert!(parse_nvidia("missing columns").is_none());
    }
}
