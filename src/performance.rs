use std::collections::VecDeque;

use slint::{Color, Image, Model, ModelRc, VecModel};

use crate::{monitor::Snapshot, Metric, Taskyy};

#[derive(Default)]
pub struct History {
    cpu: VecDeque<f32>,
    memory: VecDeque<f32>,
    received: VecDeque<f32>,
    sent: VecDeque<f32>,
    gpu: VecDeque<f32>,
}

fn push(history: &mut VecDeque<f32>, value: f32) {
    if history.len() == 60 {
        history.pop_front();
    }
    history.push_back(value);
}

fn model<T: Clone + 'static>(items: impl IntoIterator<Item = T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(items.into_iter().collect::<Vec<_>>()))
}

fn metric(title: impl Into<slint::SharedString>, detail: impl Into<slint::SharedString>) -> Metric {
    Metric { title: title.into(), detail: detail.into() }
}

fn bytes(value: f64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut scaled = value;
    let mut unit = 0;
    while scaled >= 1024.0 && unit < UNITS.len() - 1 {
        scaled /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{scaled:.0} B")
    } else {
        format!("{scaled:.1} {}", UNITS[unit])
    }
}

fn rate(value: f64) -> String {
    format!("{}/s", bytes(value))
}

fn reading(value: Option<f32>, unit: &str) -> String {
    value.filter(|value| value.is_finite()).map(|value| format!("{value:.0}{unit}"))
        .unwrap_or_else(|| "Unavailable".into())
}

pub fn trace(samples: ModelRc<f32>, maximum: f32, color: Color) -> Image {
    use std::fmt::Write;
    let count = samples.row_count().min(60);
    if count < 2 || !maximum.is_finite() || maximum <= 0.0 {
        return Image::default();
    }
    let left = (60 - count) * 10;
    let mut path = String::new();
    for (index, sample) in samples.iter().skip(samples.row_count() - count).enumerate() {
        let command = if index == 0 { 'M' } else { 'L' };
        let x = left + index * 10;
        let y = 99.0 - (sample / maximum).clamp(0.0, 1.0) * 98.0;
        let _ = write!(path, "{command} {x} {y:.2} ");
    }
    let color = format!("#{:02x}{:02x}{:02x}", color.red(), color.green(), color.blue());
    let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="590" height="100" viewBox="0 0 590 100">
        <path d="{path} L 590 100 L {left} 100 Z" fill="{color}" fill-opacity=".16"/>
        <path d="{path}" fill="none" stroke="{color}" stroke-width="1.2" stroke-linejoin="round" stroke-linecap="round"/>
    </svg>"#);
    Image::load_from_svg_data(svg.as_bytes()).unwrap_or_default()
}

impl History {
    pub fn update(&mut self, ui: &Taskyy, sample: &Snapshot) {
        push(&mut self.cpu, sample.cpu);
        if sample.total > 0 {
            push(&mut self.memory, (sample.used as f64 / sample.total as f64 * 100.0) as f32);
        }
        let external = sample.networks.iter().filter(|item| item.name != "lo");
        push(&mut self.received, external.clone().map(|item| item.rx).sum::<f64>() as f32);
        push(&mut self.sent, external.map(|item| item.tx).sum::<f64>() as f32);
        if let Some(gpu) = sample.gpus.iter().filter_map(|gpu| gpu.usage).max_by(f32::total_cmp) {
            push(&mut self.gpu, gpu);
        } else {
            self.gpu.clear();
        }

        let ceiling = self.received.iter().chain(&self.sent).copied().fold(1024.0, f32::max);
        ui.set_network_ceiling(ceiling);
        ui.set_network_axis(rate(f64::from(ceiling)).into());
        ui.set_cpu_history(model(self.cpu.iter().copied()));
        ui.set_memory_history(model(self.memory.iter().copied()));
        ui.set_rx_history(model(self.received.iter().copied()));
        ui.set_tx_history(model(self.sent.iter().copied()));
        ui.set_gpu_history(model(self.gpu.iter().copied()));

        let uptime = sample.uptime;
        ui.set_cpu_metrics(model(std::iter::once(metric(
            sample.cpu_model.as_str(),
            format!("{} logical CPUs  ·  Uptime {}h {}m  ·  Swap {} / {}",
                sample.cores.len(), uptime / 3600, uptime / 60 % 60,
                bytes(sample.swap_used as f64), bytes(sample.swap_total as f64)),
        )).chain(sample.cores.iter().map(|cpu| {
            let speed = if cpu.mhz > 0 { format!("{} MHz", cpu.mhz) } else { "Clock unavailable".into() };
            metric(cpu.name.as_str(), format!("{:.1}%  ·  {speed}", cpu.usage))
        }))));

        ui.set_gpu_metrics(model(if sample.gpus.is_empty() {
            vec![metric("No GPU detected", "No DRM device or NVIDIA driver metrics are available.")]
        } else {
            sample.gpus.iter().flat_map(|gpu| {
                let memory = match (gpu.used_memory, gpu.total_memory) {
                    (Some(used), Some(total)) => format!("{} / {}", bytes(used as f64), bytes(total as f64)),
                    _ => "Unavailable".into(),
                };
                [
                    metric(gpu.name.as_str(), format!("Utilization {}  ·  VRAM {memory}  ·  Driver {}", reading(gpu.usage, "%"), gpu.driver)),
                    metric("Sensors", {
                        let readings: Vec<_> = [
                            (gpu.temperature, "Temperature", " °C"),
                            (gpu.clock, "Clock", " MHz"),
                            (gpu.watts, "Power", " W"),
                        ].into_iter().filter_map(|(value, label, unit)| {
                            value.filter(|value| value.is_finite()).map(|value| format!("{label} {value:.0}{unit}"))
                        }).collect();
                        if readings.is_empty() { "No sensor readings exposed by the driver".into() }
                        else { readings.join("  ·  ") }
                    }),
                ]
            }).collect()
        }));
        ui.set_network_metrics(model(sample.networks.iter().map(|item| {
            metric(item.name.as_str(), format!("↓ {}  ↑ {}  ·  Total ↓ {}  ↑ {}",
                rate(item.rx), rate(item.tx), bytes(item.received as f64), bytes(item.sent as f64)))
        })));
        ui.set_disk_metrics(model(sample.disks.iter().map(|disk| {
            metric(format!("{} · {} ({})", disk.name, disk.mount, disk.filesystem),
                format!("{} / {} used  ·  Read {}  ·  Write {}",
                    bytes(disk.used as f64), bytes(disk.total as f64), rate(disk.read), rate(disk.write)))
        })));
    }
}
