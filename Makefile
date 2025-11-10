ROOT_DIR := $(shell pwd)
BUILD_DIR := target/armv7-unknown-linux-gnueabihf/release
DIST_DIR := dist
RETROARCH := third-party/RetroArch-patch
TOOLCHAIN := mholdg16/miyoomini-toolchain:latest

TARGET_TRIPLE := armv7-unknown-linux-gnueabihf
GLIBC_VERSION := 2.28

-include local.mk

.PHONY: all
all: dist build package-build $(DIST_DIR)/RetroArch/retroarch $(DIST_DIR)/.allium/bin/dufs $(DIST_DIR)/.allium/bin/syncthing $(DIST_DIR)/.allium/cores/drastic/drastic migrations strip-all

.PHONY: clean
clean:
	rm -r $(DIST_DIR) || true
	# Needs sudo because RetroArch build runs in docker as root
	sudo rm -f $(RETROARCH)/bin/retroarch_miyoo354 || true

simulator-env:
	mkdir -p simulator
	mkdir -p simulator/Roms
	mkdir -p simulator/Apps
	rsync -ar static/.allium simulator/

.PHONY: simulator
simulator: simulator-env
	RUST_LOG=debug RUST_BACKTRACE=1 ALLIUM_DATABASE=$(ROOT_DIR)/simulator/allium.db ALLIUM_BASE_DIR=$(ROOT_DIR)/simulator/.allium ALLIUM_SD_ROOT=$(ROOT_DIR)/simulator cargo run --bin $(bin) --features=simulator $(args)

.PHONY: dist
dist:
	mkdir -p $(DIST_DIR)
	rsync -a --exclude='.gitkeep' static/. $(DIST_DIR)

third-party/my283:
	wget -O third-party/my283.tar.xz https://github.com/shauninman/miyoomini-toolchain-buildroot/raw/main/support/my283.tar.xz
	cd third-party/ && tar xf my283.tar.xz
	rm third-party/my283.tar.xz

.PHONY: build
build: third-party/my283
	cargo zigbuild --release --target=$(TARGET_TRIPLE).$(GLIBC_VERSION) --features=miyoo --bin=alliumd --bin=allium-launcher --bin=allium-menu --bin=activity-tracker --bin=screenshot --bin=say --bin=show --bin=show-hotkeys --bin=myctl
	patchelf \
		--replace-needed third-party/my283/usr/lib/libcam_os_wrapper.so libcam_os_wrapper.so \
		--replace-needed third-party/my283/usr/lib/libmi_sys.so libmi_sys.so \
		target/$(TARGET_TRIPLE)/release/myctl

.PHONY: debug
debug: third-party/my283
	cargo zigbuild --target=$(TARGET_TRIPLE).$(GLIBC_VERSION) --features=miyoo --bin=alliumd --bin=allium-launcher --bin=allium-menu --bin=activity-tracker --bin=screenshot --bin=say --bin=show --bin=show-hotkeys --bin=myctl

.PHONY: strip-all
strip-all:
	find dist static migrations -type f -exec file {} \; | awk -F ':' '/not stripped/ {print $$1}' | while read f; do strip -s "$$f"; done

.PHONY: package-build
package-build:
	mkdir -p $(DIST_DIR)/.allium/bin
	rsync -a $(BUILD_DIR)/alliumd $(DIST_DIR)/.allium/bin/
	rsync -a $(BUILD_DIR)/allium-launcher $(DIST_DIR)/.allium/bin/
	rsync -a $(BUILD_DIR)/allium-menu $(DIST_DIR)/.allium/bin/
	rsync -a $(BUILD_DIR)/screenshot $(DIST_DIR)/.tmp_update/bin/
	rsync -a $(BUILD_DIR)/say $(DIST_DIR)/.tmp_update/bin/
	rsync -a $(BUILD_DIR)/show $(DIST_DIR)/.tmp_update/bin/
	rsync -a $(BUILD_DIR)/show-hotkeys $(DIST_DIR)/.tmp_update/bin/
	rsync -a $(BUILD_DIR)/activity-tracker "$(DIST_DIR)/Apps/Activity Tracker.pak/"
	rsync -a $(BUILD_DIR)/myctl $(DIST_DIR)/.tmp_update/bin/

MIGRATIONS_DIR := $(DIST_DIR)/.allium/migrations
.PHONY: migrations
migrations: $(MIGRATIONS_DIR)/0000-retroarch-config/retroarch-config.zip $(MIGRATIONS_DIR)/0001-retroarch-core-overrides/retroarch-core-overrides.zip

$(MIGRATIONS_DIR)/0000-retroarch-config/retroarch-config.zip:
	migrations/0000-retroarch-config/package.sh

$(MIGRATIONS_DIR)/0001-retroarch-core-overrides/retroarch-core-overrides.zip:
	migrations/0001-retroarch-core-overrides/package.sh

.PHONY: retroarch
retroarch: $(RETROARCH)/retroarch

$(DIST_DIR)/RetroArch/retroarch: $(RETROARCH)/bin/retroarch_miyoo354
	cp "$(RETROARCH)/bin/retroarch_miyoo354" "$(DIST_DIR)/RetroArch/retroarch"

$(RETROARCH)/bin/retroarch_miyoo354:
	docker run --rm -v /$(ROOT_DIR)/$(RETROARCH):/root/workspace $(TOOLCHAIN) bash -c "source /root/.bashrc; make all"

$(DIST_DIR)/.allium/bin/dufs:
	cd third-party/dufs && cargo zigbuild --release --target=$(TARGET_TRIPLE).$(GLIBC_VERSION)
	cp "third-party/dufs/target/$(TARGET_TRIPLE)/release/dufs" "$(DIST_DIR)/.allium/bin/"

SYNCTHING_VERSION := "v2.0.10"
SYNCTHING_URL := "https://github.com/syncthing/syncthing/releases/download/$(SYNCTHING_VERSION)/syncthing-linux-arm-$(SYNCTHING_VERSION).tar.gz"
$(DIST_DIR)/.allium/bin/syncthing:
	TEMP_DIR=$$(mktemp --directory) && \
		wget "$(SYNCTHING_URL)" -O "$$TEMP_DIR/syncthing.tar.gz" && \
		tar xf "$$TEMP_DIR/syncthing.tar.gz" --directory="$$TEMP_DIR" && \
		mv "$$TEMP_DIR/syncthing-linux-arm-$(SYNCTHING_VERSION)/syncthing" "$(DIST_DIR)/.allium/bin/syncthing" && \
		strip --strip-all "$(DIST_DIR)/.allium/bin/syncthing" || true

DRASTIC_URL := https://github.com/steward-fu/nds/releases/download/v1.8/drastic-v1.8_miyoo.zip
$(DIST_DIR)/.allium/cores/drastic/drastic:
	wget "$(DRASTIC_URL)" -O /tmp/drastic.zip
	mkdir -p $(DIST_DIR)/.allium/cores/drastic
	unzip -o /tmp/drastic.zip -d $(DIST_DIR)/.allium/cores/drastic
	rm /tmp/drastic.zip

.PHONY: lint
lint:
	cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings
	cargo fmt --all

.PHONY: bump-version
bump-version: lint
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" crates/allium-launcher/Cargo.toml
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" crates/allium-menu/Cargo.toml
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" crates/alliumd/Cargo.toml
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" crates/activity-tracker/Cargo.toml
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" crates/common/Cargo.toml
	echo "v$(version)" > static/.allium/version.txt
	cargo check
	git add crates/allium-launcher/Cargo.toml
	git add crates/allium-menu/Cargo.toml
	git add crates/alliumd/Cargo.toml
	git add crates/activity-tracker/Cargo.toml
	git add crates/common/Cargo.toml
	git add Cargo.lock
	git add static/.allium/version.txt
	git commit -m "chore: bump version to v$(version)"
	git tag "v$(version)" -a

.PHONY: deploy
deploy:
ifndef SDCARD_PATH
	$(error SDCARD_PATH is not set. Create a local.mk file with SDCARD_PATH=/path/to/sdcard or set it as an environment variable)
endif
	@echo "Deploying to $(SDCARD_PATH)..."
	rsync --progress --modify-window=1 --update --recursive --times --verbose $(DIST_DIR)/.allium $(DIST_DIR)/.tmp_update $(DIST_DIR)/Apps $(SDCARD_PATH)/
	@echo "Deployment complete! Remember to eject your SD card properly."

.PHONY: deploy-all
deploy-all:
ifndef SDCARD_PATH
	$(error SDCARD_PATH is not set. Create a local.mk file with SDCARD_PATH=/path/to/sdcard or set it as an environment variable)
endif
	@echo "Deploying full dist to $(SDCARD_PATH)..."
	rsync --progress --modify-window=1 --update --recursive --times --verbose --delete $(DIST_DIR)/.allium $(DIST_DIR)/.tmp_update $(DIST_DIR)/Apps $(SDCARD_PATH)/
	@echo "Full deployment complete! Remember to eject your SD card properly."
