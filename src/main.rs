mod hardware;
mod monitor;
mod process;
mod performance;
mod settings;

use slint::{ComponentHandle, ModelRc, Timer, TimerMode, VecModel};
use std::{cell::RefCell, rc::Rc, time::Duration};

slint::include_modules!();

fn display_summary(ui: &Taskyy, snapshot: &monitor::Snapshot) {
    ui.set_summary(format!("{:.1}% CPU   ·   {:.2} / {:.2} GiB RAM   ·   {} processes",
        snapshot.cpu, snapshot.used as f64 / 2_f64.powi(30),
        snapshot.total as f64 / 2_f64.powi(30), snapshot.processes.len()).into());
}

fn display(ui: &Taskyy, snapshot: &monitor::Snapshot) {
    let query = ui.get_query().trim().to_lowercase();
    let requested_pid = query.parse::<u32>().ok();
    let sort_column = ui.get_sort_column();
    let mut processes: Vec<_> = snapshot.processes.iter().filter(|process| {
        requested_pid.map_or_else(|| query.is_empty() || process.name.to_lowercase().contains(&query), |pid| process.pid == pid)
    }).collect();
    processes.sort_unstable_by(|left, right| {
        let order = match sort_column {
            1 => left.pid.cmp(&right.pid),
            2 => right.cpu.total_cmp(&left.cpu),
            3 => right.memory.cmp(&left.memory),
            _ => left.name.cmp(&right.name),
        };
        order.then(left.pid.cmp(&right.pid))
    });
    ui.set_rows(ModelRc::new(VecModel::from(processes.into_iter().map(|process| Row {
        name: process.name.as_str().into(),
        pid: process.pid.to_string().into(),
        cpu: format!("{:.1}%", process.cpu).into(),
        memory: format!("{:.1} MiB", process.memory as f64 / 2_f64.powi(20)).into(),
    }).collect::<Vec<_>>())));
}

fn color(value: u32) -> slint::Color {
    slint::Color::from_rgb_u8((value >> 16) as u8, (value >> 8) as u8, value as u8)
}

fn show_theme(ui: &Taskyy, theme: settings::Theme) {
    let palette = ui.global::<Palette>();
    palette.set_light(theme.light);
    palette.set_start(color(theme.start));
    palette.set_middle(color(theme.middle));
    palette.set_end(color(theme.end));
    palette.set_three_stops(theme.three_stops);
    palette.set_direction(theme.angle as f32);
    palette.set_intensity(theme.intensity as f32);
    ui.set_start_hex(settings::hex(theme.start).into());
    ui.set_middle_hex(settings::hex(theme.middle).into());
    ui.set_end_hex(settings::hex(theme.end).into());
}

fn edit_stop(ui: &Taskyy, stop: i32, text: slint::SharedString) {
    let palette = ui.global::<Palette>();
    let value = settings::parse_color(text.as_str()).map(color);
    match stop {
        0 => {
            ui.set_start_hex(text);
            if let Some(value) = value { palette.set_start(value); }
        }
        1 => {
            ui.set_middle_hex(text);
            if let Some(value) = value { palette.set_middle(value); }
        }
        _ => {
            ui.set_end_hex(text);
            if let Some(value) = value { palette.set_end(value); }
        }
    }
    ui.set_settings_status("".into());
}

fn bind_appearance(
    ui: &Taskyy,
    initial: settings::Theme,
    persist: impl Fn(settings::Theme) -> std::io::Result<()> + 'static,
) {
    let saved = Rc::new(RefCell::new(initial));
    show_theme(ui, *saved.borrow());
    let weak = ui.as_weak();
    ui.on_color_edited(move |stop, text| {
        let Some(ui) = weak.upgrade() else { return };
        edit_stop(&ui, stop, text);
    });
    let weak = ui.as_weak();
    ui.on_color_picked(move |stop, color| {
        if let Some(ui) = weak.upgrade() {
            let value = u32::from_be_bytes([0, color.red(), color.green(), color.blue()]);
            edit_stop(&ui, stop, settings::hex(value).into());
        }
    });
    let weak = ui.as_weak();
    let applied = saved.clone();
    ui.on_apply_theme(move || {
        let Some(ui) = weak.upgrade() else { return };
        let palette = ui.global::<Palette>();
        let three_stops = palette.get_three_stops();
        let start = settings::parse_color(ui.get_start_hex().as_str());
        let end = settings::parse_color(ui.get_end_hex().as_str());
        let middle = if three_stops {
            settings::parse_color(ui.get_middle_hex().as_str())
        } else {
            Some(applied.borrow().middle)
        };
        let (Some(start), Some(middle), Some(end)) = (start, middle, end) else {
            ui.set_settings_status("Enter #RRGGBB or three RGB values for each visible stop.".into());
            return;
        };
        let theme = settings::Theme {
            light: palette.get_light(), start, middle, end, three_stops,
            angle: palette.get_direction().round() as u16,
            intensity: palette.get_intensity().round() as u8,
        };
        match persist(theme) {
            Ok(()) => {
                *applied.borrow_mut() = theme;
                show_theme(&ui, theme);
                ui.set_settings_status("Theme applied.".into());
            }
            Err(error) => ui.set_settings_status(format!("Could not save theme: {error}").into()),
        }
    });
    let weak = ui.as_weak();
    ui.on_cancel_theme(move || {
        if let Some(ui) = weak.upgrade() {
            show_theme(&ui, *saved.borrow());
            ui.set_settings_status("Restored the applied theme.".into());
        }
    });
    let weak = ui.as_weak();
    ui.on_reset_theme(move || {
        if let Some(ui) = weak.upgrade() {
            show_theme(&ui, settings::Theme::default());
            ui.set_settings_status("Default theme preview.".into());
        }
    });
    let weak = ui.as_weak();
    ui.on_random_theme(move || {
        if let Some(ui) = weak.upgrade() {
            let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_nanos() as u64;
            let seed = seed ^ seed.rotate_left(21) ^ seed.rotate_right(11);
            let palette = ui.global::<Palette>();
            show_theme(&ui, settings::Theme {
                light: palette.get_light(),
                start: (seed & 0xffffff) as u32,
                middle: ((seed >> 12) & 0xffffff) as u32,
                end: ((seed >> 24) & 0xffffff) as u32,
                three_stops: palette.get_three_stops(),
                angle: (seed % 360) as u16,
                intensity: palette.get_intensity().round() as u8,
            });
            ui.set_settings_status("Random palette preview.".into());
        }
    });
}

fn bind_inspection(ui: &Taskyy, current: Rc<RefCell<Option<monitor::Snapshot>>>) {
    let weak = ui.as_weak();
    ui.on_inspect_process(move |pid| {
        let Some(ui) = weak.upgrade() else { return };
        let Ok(pid) = pid.parse::<u32>() else { return };
        let current = current.borrow();
        let Some(snapshot) = current.as_ref() else { return };
        let Some(entry) = snapshot.processes.iter().find(|entry| entry.pid == pid) else {
            ui.set_process_status("Process has exited.".into());
            return;
        };
        let unavailable = || String::from("Unavailable");
        let identity = || process::start_ticks(pid).ok();
        let mut executable = None;
        let mut arguments = None;
        if entry.start_ticks.is_some() && entry.start_ticks == identity() {
            use std::io::Read;
            executable = std::fs::read_link(format!("/proc/{pid}/exe")).ok()
                .map(|path| path.display().to_string());
            let mut cmdline = Vec::new();
            if std::fs::File::open(format!("/proc/{pid}/cmdline")).and_then(|file| {
                file.take(8192).read_to_end(&mut cmdline)
            }).is_ok() && !cmdline.is_empty() {
                let args = cmdline.split(|byte| *byte == 0).filter(|arg| !arg.is_empty())
                    .map(|arg| format!("{:?}", String::from_utf8_lossy(arg))).collect::<Vec<_>>();
                arguments = Some(args.join(" "));
            }
            if entry.start_ticks != identity() {
                executable = None;
                arguments = None;
            }
        }
        let children = snapshot.processes.iter().filter(|child| child.parent == Some(pid)).count();
        let details = [
            ("State", entry.status.clone()),
            ("Parent PID", entry.parent.map(|pid| pid.to_string()).unwrap_or_else(unavailable)),
            ("Child processes", children.to_string()),
            ("Threads", entry.thread_count.map(|count| count.to_string()).unwrap_or_else(unavailable)),
            ("Running time", format!("{}m {}s", entry.run_time / 60, entry.run_time % 60)),
            ("Disk I/O totals", format!("{} KiB read · {} KiB written", entry.disk_read / 1024, entry.disk_written / 1024)),
            ("Executable", executable.unwrap_or_else(unavailable)),
            ("Command line", arguments.unwrap_or_else(unavailable)),
        ];
        ui.set_process_details(ModelRc::new(VecModel::from(details.into_iter()
            .map(|(title, detail)| Metric { title: title.into(), detail: detail.into() }).collect::<Vec<_>>())));
        ui.set_inspected_process(format!("{} · PID {pid}", entry.name).into());
    });
}

fn bind_termination(ui: &Taskyy, current: Rc<RefCell<Option<monitor::Snapshot>>>) {
    let pending = Rc::new(RefCell::new(None));
    let request = pending.clone();
    let weak = ui.as_weak();
    ui.on_request_termination(move |pid| {
        let Some(ui) = weak.upgrade() else { return };
        if request.borrow().is_some() { return; }
        let Ok(pid) = pid.parse::<u32>() else { return };
        let snapshot = current.borrow();
        let process = snapshot.as_ref().and_then(|snapshot| snapshot.processes.iter().find(|p| p.pid == pid));
        let Some((process, start)) = process.and_then(|process| process.start_ticks.map(|start| (process, start))) else {
            ui.set_process_status("Process identity is unavailable. Wait for the next sample.".into());
            return;
        };
        match process::Target::open(pid, start) {
            Ok(target) => {
                *request.borrow_mut() = Some(target);
                ui.set_pending_process(format!("{} · PID {pid}", process.name).into());
            }
            Err(error) => ui.set_process_status(format!("Cannot select PID {pid}: {error}").into()),
        }
    });
    let weak = ui.as_weak();
    ui.on_resolve_termination(move |confirmed| {
        let Some(ui) = weak.upgrade() else { return };
        let target = pending.borrow_mut().take();
        ui.set_pending_process("".into());
        if let Some(target) = target.filter(|_| confirmed) {
            let message = match target.terminate() {
                Ok(()) => "SIGTERM sent.".into(),
                Err(error) => format!("Could not end process: {error}"),
            };
            ui.set_process_status(message.into());
        }
    });
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = Taskyy::new()?;
    ui.global::<Chart>().on_trace(performance::trace);
    slint::set_xdg_app_id("io.github.taskyy.Taskyy")?;
    bind_appearance(&ui, settings::load(), settings::save);
    let latest = monitor::start();
    let mut history = performance::History::default();
    let current = Rc::new(RefCell::new(None));
    bind_termination(&ui, current.clone());
    bind_inspection(&ui, current.clone());
    let weak = ui.as_weak();
    let displayed = current.clone();
    ui.on_filter_changed(move || {
        if let (Some(ui), Some(snapshot)) = (weak.upgrade(), displayed.borrow().as_ref()) {
            display(&ui, snapshot);
        }
    });

    let weak = ui.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(200), move || {
        let next = latest.lock().expect("snapshot lock").take();
        if let (Some(ui), Some(snapshot)) = (weak.upgrade(), next) {
            history.update(&ui, &snapshot);
            display_summary(&ui, &snapshot);
            if ui.get_page() == 0 {
                display(&ui, &snapshot);
            }
            *current.borrow_mut() = Some(snapshot);
        }
    });
    ui.run()
}

#[cfg(test)]
mod ui_tests {
    use super::*;
    use slint::{
        platform::{
            software_renderer::{MinimalSoftwareWindow, RepaintBufferType},
            Platform, PointerEventButton, WindowAdapter, WindowEvent,
        },
        LogicalPosition, PhysicalSize, Rgb8Pixel,
    };

    struct Headless(Rc<MinimalSoftwareWindow>);

    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    fn preview(name: &str, size: PhysicalSize, pixels: &[Rgb8Pixel]) {
        let Some(directory) = std::env::var_os("TASKYY_TEST_PREVIEW_DIR") else { return };
        use std::{io::Write, path::PathBuf};
        let file = std::fs::File::create(PathBuf::from(directory).join(name)).unwrap();
        let mut file = std::io::BufWriter::new(file);
        writeln!(file, "P6\n{} {}\n255", size.width, size.height).unwrap();
        for pixel in pixels {
            file.write_all(&[pixel.r, pixel.g, pixel.b]).unwrap();
        }
    }

    #[test]
    fn appearance_page_renders_and_switches_to_light() {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        slint::platform::set_platform(Box::new(Headless(window.clone()))).unwrap();
        let ui = Taskyy::new().unwrap();
        ui.global::<Chart>().on_trace(performance::trace);
        let saved = Rc::new(RefCell::new(None));
        let writes = saved.clone();
        bind_appearance(&ui, settings::Theme::default(), move |theme| {
            *writes.borrow_mut() = Some(theme);
            Ok(())
        });
        ui.show().unwrap();
        let size = PhysicalSize::new(980, 750);
        window.set_size(size);
        let mut pixels = vec![Rgb8Pixel::default(); (size.width * size.height) as usize];

        let click = |x, y| {
            let position = LogicalPosition::new(x, y);
            window.dispatch_event(WindowEvent::PointerMoved { position });
            window.dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
            window.dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
        };

        click(80., 278.);
        assert!(window.draw_if_needed(|renderer| { renderer.render(&mut pixels, size.width as usize); }));
        preview("appearance-dark.ppm", size, &pixels);
        let preview_left = pixels[132 * size.width as usize + 220];
        let preview_right = pixels[132 * size.width as usize + 920];
        assert_ne!(preview_left, preview_right, "preview should contain a rendered gradient");
        click(720., 214.);
        let palette = ui.global::<Palette>();
        assert!(palette.get_light(), "light appearance control should be clickable");
        ui.invoke_color_edited(0, "#FFE6C0".into());
        palette.set_direction(225.0);
        palette.set_intensity(40.0);
        ui.invoke_apply_theme();
        let applied = saved.borrow().expect("theme should be saved");
        assert!(applied.light);
        assert_eq!((applied.start, applied.angle, applied.intensity), (0xffe6c0, 225, 40));
        ui.invoke_reset_theme();
        assert!(!palette.get_light());
        ui.invoke_cancel_theme();
        assert!(palette.get_light());
        assert_eq!(palette.get_start(), color(0xffe6c0));
        assert!(window.draw_if_needed(|renderer| { renderer.render(&mut pixels, size.width as usize); }));
        preview("appearance-light.ppm", size, &pixels);
        ui.set_summary("4.1% CPU · 8.00 / 32.00 GiB RAM · 1 process".into());
        ui.set_rows(ModelRc::new(VecModel::from(vec![Row {
            name: "Example process".into(), pid: "1242".into(),
            cpu: "2.1%".into(), memory: "15.0 MiB".into(),
        }])));
        click(80., 164.);
        assert!(window.draw_if_needed(|renderer| { renderer.render(&mut pixels, size.width as usize); }));
        preview("processes-light.ppm", size, &pixels);
        click(80., 222.);
        for (index, x) in [300., 480., 675., 860.].into_iter().enumerate() {
            click(x, 130.);
            assert_eq!(ui.get_performance_page(), index as i32, "tab {index} must be clickable");
            assert!(window.draw_if_needed(|renderer| { renderer.render(&mut pixels, size.width as usize); }));
            let before = pixels.clone();
            let samples = || ModelRc::new(VecModel::from((0..60).map(|index| (index * 13 % 80) as f32).collect::<Vec<_>>()));
            let metrics = || ModelRc::new(VecModel::from(vec![Metric {
                title: "Example device".into(), detail: "Reading from the test fixture".into(),
            }]));
            match index {
                0 => { ui.set_cpu_history(samples()); ui.set_memory_history(samples()); ui.set_cpu_metrics(metrics()); }
                1 => { ui.set_gpu_history(samples()); ui.set_gpu_metrics(metrics()); }
                2 => { ui.set_network_ceiling(100.0); ui.set_rx_history(samples()); ui.set_tx_history(samples()); ui.set_network_metrics(metrics()); }
                _ => ui.set_disk_metrics(metrics()),
            }
            assert!(window.draw_if_needed(|renderer| { renderer.render(&mut pixels, size.width as usize); }));
            let start = 175 * size.width as usize;
            if index < 3 {
                let chart = start..(390 * size.width as usize);
                assert_ne!(&before[chart.clone()], &pixels[chart], "tab {index} should draw a trace");
            }
            assert_ne!(&before[start..], &pixels[start..], "tab {index} should render its data below the tab bar");
            preview(&format!("performance-{index}.ppm"), size, &pixels);
        }

    }
}
