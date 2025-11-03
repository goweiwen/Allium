ROOT_DIR := $(shell pwd)
BUILD_DIR := target/arm-unknown-linux-gnueabihf/release
DIST_DIR := dist
RETROARCH := third-party/RetroArch-patch
TOOLCHAIN := mholdg16/miyoomini-toolchain:latest

CROSS_TARGET_TRIPLE := arm-unknown-linux-gnueabihf

-include local.mk

PLATFORM := $(shell uname -m)
ifeq ($(PLATFORM),arm64)
  export CROSS_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_IMAGE_TOOLCHAIN = aarch64-unknown-linux-gnu
  export CROSS_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_IMAGE = goweiwen/cross-with-clang_arm-unknown-linux-gnueabihf:aarch64
endif

.PHONY: all
all: dist build package-build $(DIST_DIR)/RetroArch/retroarch $(DIST_DIR)/.allium/bin/dufs $(DIST_DIR)/.allium/bin/syncthing $(DIST_DIR)/.allium/cores/drastic/launch.sh migrations

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
	cross build --release --target=$(CROSS_TARGET_TRIPLE) --features=miyoo --bin=alliumd --bin=allium-launcher --bin=allium-menu --bin=activity-tracker --bin=screenshot --bin=say --bin=show --bin=show-hotkeys --bin=myctl

.PHONY: debug
debug: third-party/my283
	cross build --target=$(CROSS_TARGET_TRIPLE) --features=miyoo --bin=alliumd --bin=allium-launcher --bin=allium-menu --bin=activity-tracker --bin=screenshot --bin=say --bin=show --bin=show-hotkeys --bin=myctl

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
	docker run --rm -v /$(ROOT_DIR)/$(RETROARCH):/root/workspace $(TOOLCHAIN) bash -c "source /root/.bashrc; make clean all"

$(DIST_DIR)/.allium/bin/dufs:
	cd third-party/dufs && cross build --release --target=$(CROSS_TARGET_TRIPLE)
	cp "third-party/dufs/target/$(CROSS_TARGET_TRIPLE)/release/dufs" "$(DIST_DIR)/.allium/bin/"

$(DIST_DIR)/.allium/bin/syncthing:
	cd "$$(mktemp --directory)"
	wget "https://github.com/syncthing/syncthing/releases/download/v2.0.10/syncthing-linux-arm-v2.0.10.tar.gz" -O syncthing.tar.gz
	tar xf syncthing.tar.gz
	mv "syncthing-linux-arm-v2.0.10/syncthing" "$(DIST_DIR)/.allium/bin/syncthing"
	strip -s "$(DIST_DIR)/.allium/bin/syncthing" || true

DRASTIC_URL := https://github.com/steward-fu/nds/releases/download/v1.8/drastic-v1.8_miyoo.zip
$(DIST_DIR)/.allium/cores/drastic/launch.sh:
	wget "$(DRASTIC_URL)" -O /tmp/drastic.zip
	mkdir -p $(DIST_DIR)/.allium/cores/drastic/drastic
	unzip -o /tmp/drastic.zip -d $(DIST_DIR)/.allium/cores/drastic/drastic
	rm /tmp/drastic.zip

.PHONY: lint
lint:
	cargo fmt --all -- --check
	cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings

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
	rsync -av --delete $(DIST_DIR)/.allium $(DIST_DIR)/.tmp_update $(DIST_DIR)/Apps $(SDCARD_PATH)/
	@echo "Deployment complete! Remember to eject your SD card properly."

.PHONY: deploy-all
deploy-all:
ifndef SDCARD_PATH
	$(error SDCARD_PATH is not set. Create a local.mk file with SDCARD_PATH=/path/to/sdcard or set it as an environment variable)
endif
	@echo "Deploying full dist to $(SDCARD_PATH)..."
	rsync -av --delete $(DIST_DIR)/ $(SDCARD_PATH)/
	@echo "Full deployment complete! Remember to eject your SD card properly."
