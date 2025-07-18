# Build variables
BUILD_TYPE ?= release
TARGET ?= $(shell rustc -vV | grep "host" | sed 's/host: //')
ETC_DIR ?= etc

.PHONY_: install_pam_login_ng
install_pam_login_ng: target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-service target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-mount target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so
	install -D -m 755 target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-service $(PREFIX)/usr/bin/pam_login_ng-service
	install -D -m 755 target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-mount $(PREFIX)/usr/bin/pam_login_ng-mount
	install -D -m 755 target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so $(PREFIX)/usr/lib/security/pam_login_ng.so
	install -D -m 644 rootfs/usr/lib/systemd/system/pam_login_ng.service $(PREFIX)/usr/lib/systemd/system/pam_login_ng.service
	install -D -m 644 rootfs/usr/share/dbus-1/system.d/org.neroreflex.login_ng_mount.conf $(PREFIX)/usr/share/dbus-1/system.d/org.neroreflex.login_ng_mount.conf
	install -D -m 644 rootfs/usr/share/dbus-1/system.d/org.neroreflex.login_ng_session.conf $(PREFIX)/usr/share/dbus-1/system.d/org.neroreflex.login_ng_session.conf

.PHONY: install
install: install_pam_login_ng

.PHONY: build
build: target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-service target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-mount

.PHONY: fetch
fetch: Cargo.lock
	cargo fetch --locked

target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-mount: target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so
	cargo build --frozen --offline --all-features --$(BUILD_TYPE) --bin pam_login_ng-mount --target=$(TARGET) --target-dir target

target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-service: target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so
	cargo build --frozen --offline --all-features --$(BUILD_TYPE) --bin pam_login_ng-service --target=$(TARGET) --target-dir target

target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so: fetch
	cargo build --frozen --offline --all-features --$(BUILD_TYPE) --lib --target=$(TARGET) --target-dir target

.PHONY: clean
clean:
	cargo clean

.PHONY: all
all: build

.PHONY: deb
deb: fetch
	cargo-deb --all-features
