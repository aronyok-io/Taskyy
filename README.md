# Taskyy

<img src="assets/taskyy.png" alt="Taskyy penguin" width="96">

Taskyy is a native Linux system monitor and task manager built with Rust and Slint. It runs locally and stores appearance settings in your user configuration directory.

[Discussions](https://github.com/aronyok-io/Taskyy/discussions) · [Report a bug](https://github.com/aronyok-io/Taskyy/issues/new/choose) · [Contributing](CONTRIBUTING.md)

## Install on Fedora 44 KDE (x86_64)

Download the x86_64 RPM from [GitHub Releases](https://github.com/aronyok-io/Taskyy/releases/latest). Open it in Discover or install it from the download directory:

```sh
sudo dnf install ./taskyy-0.2.4-1.fc44.x86_64.rpm
```

Then open **Taskyy** from the KDE application launcher. If you pinned a source-run instance earlier, unpin that entry and pin Taskyy from the launcher. The desktop file and window both use the application ID `io.github.taskyy.Taskyy`.

Remove the package with `sudo dnf remove taskyy`.

## Use Taskyy

- **Processes** samples the process table every second. Search by name or enter a PID for an exact match. Click a column heading to sort or click a process name to inspect its state, parent, threads, executable and command line where readable. CPU percentages are reported per logical CPU and may exceed 100%. Memory is resident memory.
- **End…** opens a confirmation showing the target name and PID. **End process** sends SIGTERM through a Linux pidfd. **Cancel** closes the confirmation without sending a signal. Taskyy runs with the caller's permissions.
- **Performance** shows CPU, RAM and swap use, per-logical-CPU readings, and system uptime. RAM use is total minus available memory.
- **GPU** reads NVIDIA driver metrics through `nvidia-smi` when available and reads exposed Linux DRM/sysfs sensors for other devices. On hybrid systems, devices are listed separately. Unsupported or inaccessible readings are marked unavailable.
- **Network** shows per-interface receive and send rates and totals. The history charts aggregate interfaces other than loopback.
- **Storage** shows mounted filesystems, capacity, and read and write rates exposed through sysinfo. Multiple mount points can share a device.
- **Appearance** previews light and dark palettes and blends a two- or three-stop gradient over the workspace. Select a stop and use the hue slider and saturation/brightness picker, or enter a HEX or comma-separated RGB value. Direction rotates the gradient. Intensity controls its opacity over the base palette. **Apply** saves to `${XDG_CONFIG_HOME:-$HOME/.config}/taskyy/appearance-v2`. **Cancel** restores the saved theme. **Reset** previews the defaults.

## Build from source

```sh
sudo dnf install cargo gcc pkgconf-pkg-config fontconfig-devel
cargo test --locked
cargo run --locked
```

Build a Fedora RPM with:

```sh
sudo dnf install rpm-build desktop-file-utils ImageMagick
./packaging/build-rpm.sh
```

The build script puts the package in `dist/` by default. It compiles a release executable, embeds the in-app icon, and packages a matching desktop entry and hicolor icon.

## Smoke test

1. Open Taskyy from KDE and check the penguin in the launcher and taskbar.
2. Open Performance. Switch between CPU & RAM, GPU, Network and Storage. Generate network traffic and check the receive/send rates. Compare available GPU readings with your driver tools.
3. Change the gradient, switch to Light, apply, and reopen Taskyy. Verify that the theme persists. Make another change and cancel it.
4. Run `sleep 60 & echo $!` in a terminal. Find that PID in Taskyy, choose **End…**, then cancel. Repeat and confirm. The disposable process should disappear from the table.

The remaining Windows Task Manager parity work includes grouped process trees, per-process GPU and network attribution, startup management, signed-in user sessions, systemd service controls, application history, and process priority and additional signal controls. Theme preset management and theme import/export also remain on the roadmap. See [the technical research](RESEARCH.md) and [the license](LICENSE).
