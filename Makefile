# Build variables
BUILD_TYPE ?= release
TARGET ?= $(shell rustc -vV | grep "host" | sed 's/host: //')
ETC_DIR ?= etc

.PHONY_: install_pam_polyauth
install_pam_polyauth: target/$(TARGET)/$(BUILD_TYPE)/pam_polyauth-service target/$(TARGET)/$(BUILD_TYPE)/polyauthctl target/$(TARGET)/$(BUILD_TYPE)/libpam_polyauth.so
	install -D -m 755 target/$(TARGET)/$(BUILD_TYPE)/pam_polyauth-service $(PREFIX)/usr/bin/pam_polyauth-service
	install -D -m 755 target/$(TARGET)/$(BUILD_TYPE)/polyauthctl $(PREFIX)/usr/bin/polyauthctl
	install -D -m 755 target/$(TARGET)/$(BUILD_TYPE)/libpam_polyauth.so $(PREFIX)/usr/lib/security/pam_polyauth.so
	install -D -m 644 rootfs/usr/lib/systemd/system/pam_polyauth.service $(PREFIX)/usr/lib/systemd/system/pam_polyauth.service
	install -D -m 644 rootfs/usr/share/dbus-1/system.d/org.neroreflex.polyauth_mount.conf $(PREFIX)/usr/share/dbus-1/system.d/org.neroreflex.polyauth_mount.conf
	install -D -m 644 rootfs/usr/share/dbus-1/system.d/org.neroreflex.polyauth_session.conf $(PREFIX)/usr/share/dbus-1/system.d/org.neroreflex.polyauth_session.conf
	install -D -m 644 Manual/polyauthctl.1 $(PREFIX)/usr/share/man/man1/polyauthctl.1
	install -D -m 644 completions/polyauthctl.bash $(PREFIX)/usr/share/bash-completion/completions/polyauthctl
	install -D -m 644 completions/polyauthctl.zsh $(PREFIX)/usr/share/zsh/site-functions/_polyauthctl

.PHONY: install
install: install_pam_polyauth

.PHONY: build
build: target/$(TARGET)/$(BUILD_TYPE)/libpam_polyauth.so target/$(TARGET)/$(BUILD_TYPE)/pam_polyauth-service target/$(TARGET)/$(BUILD_TYPE)/polyauthctl

.PHONY: fetch
fetch: Cargo.lock
	cargo fetch --locked

target/$(TARGET)/$(BUILD_TYPE)/pam_polyauth-service: target/$(TARGET)/$(BUILD_TYPE)/libpam_polyauth.so
	cargo build --frozen --offline --all-features --$(BUILD_TYPE) --bin pam_polyauth-service --target=$(TARGET) --target-dir target

target/$(TARGET)/$(BUILD_TYPE)/polyauthctl: target/$(TARGET)/$(BUILD_TYPE)/libpam_polyauth.so
	cargo build --frozen --offline --all-features --$(BUILD_TYPE) --bin polyauthctl --target=$(TARGET) --target-dir target

target/$(TARGET)/$(BUILD_TYPE)/libpam_polyauth.so: fetch
	cargo build --frozen --offline --all-features --$(BUILD_TYPE) --lib --target=$(TARGET) --target-dir target

.PHONY: clean
clean:
	cargo clean

.PHONY: all
all: build

.PHONY: deb
deb: fetch
	cargo-deb --all-features
