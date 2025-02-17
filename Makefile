# Build variables
BUILD_TYPE ?= release

.PHONY: install
install: build
	install -D -m 755 target/$(BUILD_TYPE)/login_ng-ctl $(PREFIX)/usr/bin/login_ng-ctl
	install -D -m 755 target/$(BUILD_TYPE)/login_ng-cli $(PREFIX)/usr/bin/login_ng-cli
	install -D -m 755 target/$(BUILD_TYPE)/pam_login_ng-service $(PREFIX)/usr/bin/pam_login_ng-service
	install -D -m 755 target/$(BUILD_TYPE)/libpam_login_ng.so $(PREFIX)/usr/lib/security/pam_login_ng.so
	install -D -m 644 rootfs/usr/lib/systemd/system/pam_login_ng.service $(PREFIX)/usr/lib/systemd/system/pam_login_ng.service
	install -D -m 644 rootfs/usr/lib/systemd/system/login_ng.service $(PREFIX)/usr/lib/systemd/system/login_ng.service
	install -D -m 644 rootfs/usr/lib/sysusers.d/login_ng.conf $(PREFIX)/usr/lib/sysusers.d/login_ng.conf

.PHONY: build
build: target/$(BUILD_TYPE)/login_ng-cli target/$(BUILD_TYPE)/login_ng-ctl target/$(BUILD_TYPE)/pam_login_ng-service target/$(BUILD_TYPE)/libpam_login_ng.so

target/$(BUILD_TYPE)/login_ng-cli:
	cd login_ng-cli && cargo build --frozen --offline --$(BUILD_TYPE)

target/$(BUILD_TYPE)/login_ng-ctl:
	cd login_ng-ctl && cargo build --frozen --offline --$(BUILD_TYPE)

target/$(BUILD_TYPE)/pam_login_ng-service: target/$(BUILD_TYPE)/libpam_login_ng.so
	cd pam_login_ng && cargo build --frozen --offline --$(BUILD_TYPE) --bin pam_login_ng-service

target/$(BUILD_TYPE)/libpam_login_ng.so:
	cd pam_login_ng && cargo build --frozen --offline --$(BUILD_TYPE) --lib

.PHONY: clean
clean:
	cargo clean
	cd login_ng-cli && cargo clean
	cd login_ng-ctl && cargo clean
	cd pam_login_ng && cargo clean

.PHONY: all
all: build