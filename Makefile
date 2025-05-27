# Build variables
BUILD_TYPE ?= release

.PHONY: install
install: build
	install -D -m 755 sessionexec/target/$(BUILD_TYPE)/sessionexec $(PREFIX)/usr/bin/sessionexec
	install -D -m 755 login_ng-ctl/target/$(BUILD_TYPE)/login_ng-ctl $(PREFIX)/usr/bin/login_ng-ctl
	install -D -m 755 login_ng-cli/target/$(BUILD_TYPE)/login_ng-cli $(PREFIX)/usr/bin/login_ng-cli
	install -D -m 755 login_ng-session/target/$(BUILD_TYPE)/login_ng-session $(PREFIX)/usr/bin/login_ng-session
	install -D -m 755 login_ng-session/target/$(BUILD_TYPE)/login_ng-sessionctl $(PREFIX)/usr/bin/login_ng-sessionctl
	install -D -m 755 pam_login_ng/target/$(BUILD_TYPE)/pam_login_ng-service $(PREFIX)/usr/bin/pam_login_ng-service
	install -D -m 755 pam_login_ng/target/$(BUILD_TYPE)/libpam_login_ng.so $(PREFIX)/usr/lib/security/pam_login_ng.so
	install -D -m 755 rootfs/usr/share/wayland-sessions/login_ng-session.desktop $(PREFIX)/usr/share/wayland-sessions/login_ng-session.desktop
	install -D -m 755 rootfs/usr/share/wayland-sessions/game-mode.desktop $(PREFIX)/usr/share/wayland-sessions/game-mode.desktop
	install -D -m 755 rootfs/usr/share/applications/org.sessionexec.session-return.desktop $(PREFIX)/usr/share/applications/org.sessionexec.session-return.desktop
	ln -s /usr/share/wayland-sessions/game-mode.desktop $(PREFIX)/usr/share/wayland-sessions/default.desktop
	install -D -m 755 rootfs/usr/bin/start-login_ng-session $(PREFIX)/usr/bin/start-login_ng-session
	install -D -m 644 rootfs/usr/lib/systemd/system/pam_login_ng.service $(PREFIX)/usr/lib/systemd/system/pam_login_ng.service
	install -D -m 644 rootfs/usr/lib/systemd/system/login_ng@.service $(PREFIX)/usr/lib/systemd/system/login_ng@.service
	install -D -m 644 rootfs/usr/lib/sysusers.d/login_ng.conf $(PREFIX)/usr/lib/sysusers.d/login_ng.conf
	install -D -m 755 rootfs/usr/lib/sessionexec/restart_session.sh $(PREFIX)/usr/lib/sessionexec/restart_session.sh
	install -D -m 755 rootfs/usr/lib/sessionexec/session-return.sh $(PREFIX)/usr/lib/sessionexec/session-return.sh
	install -D -m 755 rootfs/usr/lib/os-session-select $(PREFIX)/usr/lib/os-session-select
	install -D -m 644 rootfs/etc/pam.d/login_ng $(PREFIX)/etc/pam.d/login_ng
	install -D -m 644 rootfs/etc/pam.d/login_ng-autologin $(PREFIX)/etc/pam.d/login_ng-autologin
	install -D -m 644 rootfs/etc/pam.d/login_ng-ctl $(PREFIX)/etc/pam.d/login_ng-ctl
	install -D -m 644 rootfs/etc/login_ng-session/steamdeck.service $(PREFIX)/etc/login_ng-session/steamdeck.service
	install -D -m 644 rootfs/etc/login_ng-session/default.service $(PREFIX)/etc/login_ng-session/default.service

.PHONY: build
build: fetch login_ng-cli/target/$(BUILD_TYPE)/sessionexec login_ng-cli/target/$(BUILD_TYPE)/login_ng-cli login_ng-ctl/target/$(BUILD_TYPE)/login_ng-ctl login_ng-gui/target/$(BUILD_TYPE)/login_ng-gui login_ng-session/target/$(BUILD_TYPE)/login_ng-session login_ng-session/target/$(BUILD_TYPE)/login_ng-sessionctl pam_login_ng/target/$(BUILD_TYPE)/pam_login_ng-service pam_login_ng/target/$(BUILD_TYPE)/libpam_login_ng.so

.PHONY: fetch
fetch: Cargo.lock
	cargo fetch --locked

login_ng-cli/target/$(BUILD_TYPE)/sessionexec:
	cd sessionexec && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --target-dir target

login_ng-cli/target/$(BUILD_TYPE)/login_ng-cli:
	cd login_ng-cli && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --target-dir target

login_ng-ctl/target/$(BUILD_TYPE)/login_ng-ctl:
	cd login_ng-ctl && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --target-dir target

login_ng-gui/target/$(BUILD_TYPE)/login_ng-gui:
	cd login_ng-gui && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --target-dir target

login_ng-session/target/$(BUILD_TYPE)/login_ng-session:
	cd login_ng-session && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --target-dir target

login_ng-session/target/$(BUILD_TYPE)/login_ng-sessionctl:
	cd login_ng-session && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --target-dir target --bin login_ng-sessionctl

pam_login_ng/target/$(BUILD_TYPE)/pam_login_ng-service: pam_login_ng/target/$(BUILD_TYPE)/libpam_login_ng.so
	cd pam_login_ng && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --bin pam_login_ng-service --target-dir target

pam_login_ng/target/$(BUILD_TYPE)/libpam_login_ng.so:
	cd pam_login_ng && cargo build --frozen --offline --all-features --$(BUILD_TYPE) --lib --target-dir target

.PHONY: clean
clean:
	cargo clean
	rm -rf login_ng/target
	rm -rf login_ng-cli/target
	rm -rf login_ng-gui/target
	rm -rf login_ng-ctl/target
	rm -rf login_ng-session/target
	rm -rf pam_login_ng-common/target
	rm -rf pam_login_ng/target
	rm -rf sessionexec/target

.PHONY: all
all: build

.PHONY: deb
deb: fetch
	cd sessionexec && cargo-deb --all-features
	cd login_ng-cli && cargo-deb --all-features
	cd login_ng-ctl && cargo-deb --all-features
	cd login_ng-gui && cargo-deb --all-features
	cd login_ng-session && cargo-deb --all-features
	cd pam_login_ng && cargo-deb --all-features
