use std::{
    env, fs,
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub light: bool,
    pub start: u32,
    pub middle: u32,
    pub end: u32,
    pub three_stops: bool,
    pub angle: u16,
    pub intensity: u8,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            light: false,
            start: 0x7951ce,
            middle: 0xa84b9b,
            end: 0x248c8b,
            three_stops: false,
            angle: 135,
            intensity: 48,
        }
    }
}

pub fn hex(value: u32) -> String {
    format!("#{value:06X}")
}

pub fn parse_color(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.contains(',') {
        let mut channels = text.split(',').map(|channel| channel.trim().parse::<u8>().ok());
        let [red, green, blue] = [channels.next()??, channels.next()??, channels.next()??];
        return channels.next().is_none().then_some(u32::from_be_bytes([0, red, green, blue]));
    }
    let digits = text.strip_prefix('#').unwrap_or(text);
    (digits.len() == 6 && digits.bytes().all(|digit| digit.is_ascii_hexdigit()))
        .then(|| u32::from_str_radix(digits, 16).expect("validated hex"))
}

impl Theme {
    fn encode(self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            u8::from(self.light), hex(self.start), hex(self.middle), hex(self.end),
            u8::from(self.three_stops), self.angle, self.intensity,
        )
    }

    fn decode(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        let theme = Self {
            light: parse_toggle(lines.next()?)?,
            start: parse_color(lines.next()?)?,
            middle: parse_color(lines.next()?)?,
            end: parse_color(lines.next()?)?,
            three_stops: parse_toggle(lines.next()?)?,
            angle: lines.next()?.parse().ok()?,
            intensity: lines.next()?.parse().ok()?,
        };
        (theme.angle <= 360 && theme.intensity <= 100 && lines.next().is_none()).then_some(theme)
    }
}

fn parse_toggle(value: &str) -> Option<bool> {
    match value {
        "0" => Some(false),
        "1" => Some(true),
        _ => None,
    }
}

fn path() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|base| base.join("taskyy").join("appearance-v2"))
}

fn read_settings(path: &Path) -> io::Result<String> {
    const LIMIT: u64 = 4096;
    let file = fs::OpenOptions::new().read(true).custom_flags(libc::O_NONBLOCK).open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "settings must be a regular file"));
    }
    let mut text = String::new();
    file.take(LIMIT + 1).read_to_string(&mut text)?;
    if text.len() as u64 > LIMIT {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "settings file is too large"));
    }
    Ok(text)
}

pub fn load() -> Theme {
    let Some(path) = path() else { return Theme::default() };
    match read_settings(&path) {
        Ok(text) => Theme::decode(&text).unwrap_or_default(),
        Err(_) => {
            let mut theme = Theme::default();
            if let Ok(legacy) = read_settings(&path.with_file_name("appearance-v1")) {
                let mut lines = legacy.lines();
                theme.light = lines.next() == Some("light");
                theme.start = match lines.next() {
                    Some("1") => 0x248c8b,
                    Some("2") => 0xba4b78,
                    _ => theme.start,
                };
            }
            theme
        }
    }
}

pub fn save(theme: Theme) -> io::Result<()> {
    let path = path().ok_or_else(|| io::Error::other("configuration directory unavailable"))?;
    let dir = path.parent().expect("settings directory");
    fs::create_dir_all(dir)?;
    let temporary = dir.join(format!(".appearance-{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&temporary)?;
    let result = file.write_all(theme.encode().as_bytes())
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_round_trip_and_validation() {
        let theme = Theme { light: true, angle: 225, intensity: 40, three_stops: true, ..Theme::default() };
        assert_eq!(Theme::decode(&theme.encode()), Some(theme));
        assert_eq!(Theme::decode(&theme.encode().replace("225", "361")), None);
        assert_eq!(parse_color("#12X456"), None);
        assert_eq!(parse_color("#FFE6C0"), Some(0xffe6c0));
        assert_eq!(parse_color("255, 230, 192"), Some(0xffe6c0));
        assert_eq!(parse_color("256, 230, 192"), None);
    }
}
