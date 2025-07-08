# Build variables
BUILD_TYPE ?= release
TARGET ?= $(shell rustc -vV | grep "host" | sed 's/host: //')
ETC_DIR ?= etc

.PHONY_: install_pam_login_ng
install_pam_login_ng: pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-service pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-mount pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so
	install -D -m 755 pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-service $(PREFIX)/usr/bin/pam_login_ng-service
	install -D -m 755 pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-mount $(PREFIX)/usr/bin/pam_login_ng-mount
	install -D -m 755 pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so $(PREFIX)/usr/lib/security/pam_login_ng.so
	install -D -m 644 rootfs/usr/lib/systemd/system/pam_login_ng.service $(PREFIX)/usr/lib/systemd/system/pam_login_ng.service

.PHONY: install
install: install_pam_login_ng

.PHONY: build
build: pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-service pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-mount

.PHONY: fetch
fetch: Cargo.lock
	cargo fetch --locked

pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-mount: pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so
	cd pam_login_ng && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --bin pam_login_ng-mount --target=$(TARGET) --target-dir target

pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/pam_login_ng-service: pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so
	cd pam_login_ng && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --bin pam_login_ng-service --target=$(TARGET) --target-dir target

pam_login_ng/target/$(TARGET)/$(BUILD_TYPE)/libpam_login_ng.so: fetch
	cd pam_login_ng && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --lib --target=$(TARGET) --target-dir target

.PHONY: clean
clean:
	cargo clean
	rm -rf pam_login_ng-common/target
	rm -rf pam_login_ng/target

.PHONY: all
all: build

.PHONY: deb
deb: fetch
	cd pam_login_ng && cargo-deb --all-features
