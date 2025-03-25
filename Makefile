# Build variables
BUILD_TYPE ?= release

.PHONY: install
install: build
	install -D -m 755 login_ng-ctl/target/$(BUILD_TYPE)/login_ng-ctl $(PREFIX)/usr/bin/login_ng-ctl
	install -D -m 755 login_ng-cli/target/$(BUILD_TYPE)/login_ng-cli $(PREFIX)/usr/bin/login_ng-cli
	install -D -m 755 pam_login_ng/target/$(BUILD_TYPE)/pam_login_ng-service $(PREFIX)/usr/bin/pam_login_ng-service
	install -D -m 755 pam_login_ng/target/$(BUILD_TYPE)/libpam_login_ng.so $(PREFIX)/usr/lib/security/pam_login_ng.so
	install -D -m 644 rootfs/usr/lib/systemd/system/pam_login_ng.service $(PREFIX)/usr/lib/systemd/system/pam_login_ng.service
	install -D -m 644 rootfs/usr/lib/systemd/system/login_ng@.service $(PREFIX)/usr/lib/systemd/system/login_ng@.service
	install -D -m 644 rootfs/usr/lib/sysusers.d/login_ng.conf $(PREFIX)/usr/lib/sysusers.d/login_ng.conf
	install -D -m 644 rootfs/etc/pam.d/login_ng $(PREFIX)/etc/pam.d/login_ng
	install -D -m 644 rootfs/etc/pam.d/login_ng-autologin $(PREFIX)/etc/pam.d/login_ng-autologin
	install -D -m 644 rootfs/etc/pam.d/login_ng-ctl $(PREFIX)/etc/pam.d/login_ng-ctl

.PHONY: build
build: fetch login_ng-cli/target/$(BUILD_TYPE)/login_ng-cli login_ng-ctl/target/$(BUILD_TYPE)/login_ng-ctl pam_login_ng/target/$(BUILD_TYPE)/pam_login_ng-service pam_login_ng/target/$(BUILD_TYPE)/libpam_login_ng.so

.PHONY: fetch
fetch: Cargo.lock
	cargo fetch --locked

login_ng-cli/target/$(BUILD_TYPE)/login_ng-cli:
	cd login_ng-cli && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --target-dir target

login_ng-ctl/target/$(BUILD_TYPE)/login_ng-ctl:
	cd login_ng-ctl && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --target-dir target

pam_login_ng/target/$(BUILD_TYPE)/pam_login_ng-service: pam_login_ng/target/$(BUILD_TYPE)/libpam_login_ng.so
	cd pam_login_ng && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --bin pam_login_ng-service --target-dir target

pam_login_ng/target/$(BUILD_TYPE)/libpam_login_ng.so:
	cd pam_login_ng && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --lib --target-dir target

.PHONY: clean
clean:
	cargo clean
	rm -rf login_ng-cli/target
	rm -rf login_ng-ctl/target
	rm -rf pam_login_ng/target

.PHONY: all
all: build

.PHONY: deb
deb: fetch
	cd login_ng-cli && cargo-deb --all-features
	cd login_ng-ctl && cargo-deb --all-features
	cd pam_login_ng && cargo-deb --all-features
