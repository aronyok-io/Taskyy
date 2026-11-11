# Technical references

[TuxManager](https://github.com/benapetr/TuxManager) provides a reference for Windows-style process navigation on Linux. [Plasma System Monitor](https://github.com/KDE/plasma-systemmonitor) separates sensor presentation from collection through KDE infrastructure. [evildojo-tmog](https://github.com/mikedesu/evildojo-tmog) illustrates modular Linux collectors backed by procfs and sysfs. [GNOME System Monitor](https://github.com/GNOME/gnome-system-monitor) and [Mission Center](https://gitlab.com/mission-center-devs/mission-center) provide additional Linux desktop interface references.

Taskyy uses [sysinfo](https://github.com/GuillaumeGomez/sysinfo) for CPU, memory and process snapshots. The monitor reuses a System instance, primes CPU counters, and reports used RAM as total minus available memory. Linux-specific integrations include pidfds for race-resistant signaling, sysfs and vendor APIs for GPU sensors, XDG autostart entries, and systemd D-Bus for services.


The UI uses Slint's build-time compiler and virtualized ListView. Sampling runs on a worker, which publishes the latest snapshot. Graphs keep 60 samples in memory.

These projects were used as references. Taskyy source is GPL-3.0-only. See [LICENSE](LICENSE).
