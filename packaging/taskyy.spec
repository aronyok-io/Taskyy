Name:           taskyy
Version:        0.2.4
Release:        1%{?dist}
Summary:        Desktop system monitor and task manager
License:        GPL-3.0-only
Source0:        taskyy-payload.tar.gz

%global debug_package %{nil}

%description
Taskyy is a desktop process manager and system monitor with live CPU, memory, GPU, network and storage
graphs and a customizable gradient appearance.

%prep
%setup -q -n taskyy-payload

%install
install -Dm0755 taskyy %{buildroot}%{_bindir}/taskyy
install -Dm0644 io.github.taskyy.Taskyy.desktop %{buildroot}%{_datadir}/applications/io.github.taskyy.Taskyy.desktop
install -Dm0644 icon.png %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/io.github.taskyy.Taskyy.png

%files
%license LICENSE
%{_bindir}/taskyy
%{_datadir}/applications/io.github.taskyy.Taskyy.desktop
%{_datadir}/icons/hicolor/256x256/apps/io.github.taskyy.Taskyy.png

%changelog
* Thu Nov 12 2026 Taskyy contributors - 0.2.4-1
- Reduce background UI work and harden local settings handling.
