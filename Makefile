ROOT_DIR := $(shell pwd)
BUILD_DIR := target/arm-unknown-linux-gnueabihf/release
DIST_DIR := dist
RETROARCH := third-party/RetroArch
TOOLCHAIN := mholdg16/miyoomini-toolchain:latest

PLATFORM := $(shell uname -m)
ifeq ($(PLATFORM),arm64)
  export CROSS_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_IMAGE_TOOLCHAIN = aarch64-unknown-linux-gnu
  export CROSS_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_IMAGE = goweiwen/cross-with-clang_arm-unknown-linux-gnueabihf:aarch64
endif

all: static build package-build package-retroarch migrations

simulator-env:
	mkdir -p assets/simulator
	mkdir -p assets/simulator/Roms
	mkdir -p assets/simulator/Apps
	rsync -ar assets/root/.allium assets/simulator/

simulator-launcher: simulator-env
	RUST_LOG=trace RUST_BACKTRACE=1 ALLIUM_DATABASE=assets/simulator/allium.db ALLIUM_BASE_DIR=assets/simulator/.allium ALLIUM_SD_ROOT=assets/simulator cargo run --features=simulator --bin allium-launcher

simulator-menu: simulator-env
	RUST_LOG=trace RUST_BACKTRACE=1 ALLIUM_DATABASE=assets/simulator/allium.db ALLIUM_BASE_DIR=assets/simulator/.allium ALLIUM_SD_ROOT=assets/simulator cargo run --features=simulator --bin allium-menu

simulator: simulator-env
	RUST_LOG=trace RUST_BACKTRACE=1 ALLIUM_DATABASE=assets/simulator/allium.db ALLIUM_BASE_DIR=assets/simulator/.allium ALLIUM_SD_ROOT=assets/simulator cargo run --bin $(bin) --features=simulator $(args)

clean:
	rm -r $(DIST_DIR)
	rm -f $(RETROARCH)/retroarch

static:
	mkdir -p $(DIST_DIR)
	rsync -a --exclude='.gitkeep' assets/root/. $(DIST_DIR)

third-party/my283:
	wget -O third-party/my283.tar.xz https://github.com/shauninman/miyoomini-toolchain-buildroot/raw/main/support/my283.tar.xz
	cd third-party/ && tar xf my283.tar.xz
	rm third-party/my283.tar.xz

build: third-party/my283
	cross build --release --features=miyoo --bin=alliumd --bin=allium-launcher --bin=allium-menu --bin=activity-tracker --bin=screenshot --bin=say --bin=show --bin=show-hotkeys --bin=myctl

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
migrations: $(MIGRATIONS_DIR)/0000-retroarch-config/retroarch-config.zip $(MIGRATIONS_DIR)/0001-retroarch-core-overrides/retroarch-core-overrides.zip $(MIGRATIONS_DIR)/0002-drastic/drastic.zip

$(MIGRATIONS_DIR)/0000-retroarch-config/retroarch-config.zip:
	assets/migrations/0000-retroarch-config/package.sh

$(MIGRATIONS_DIR)/0001-retroarch-core-overrides/retroarch-core-overrides.zip:
	assets/migrations/0001-retroarch-core-overrides/package.sh

$(MIGRATIONS_DIR)/0002-drastic/drastic.zip:
	assets/migrations/0002-drastic/package.sh

retroarch: $(RETROARCH)/retroarch

package-retroarch: retroarch
	rsync -a $(RETROARCH)/retroarch "$(DIST_DIR)/RetroArch"

$(RETROARCH)/retroarch:
	docker run --rm -v /$(ROOT_DIR)/third-party:/root/workspace $(TOOLCHAIN) bash -c "source /root/.bashrc; cd RetroArch; make clean all ADD_NETWORKING=1 PACKAGE_NAME=retroarch"

lint:
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged --all-targets

bump-version: lint
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" allium-launcher/Cargo.toml
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" allium-menu/Cargo.toml
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" alliumd/Cargo.toml
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" activity-tracker/Cargo.toml
	sed -i'' -e "s/^version = \".*\"/version = \"$(version)\"/" common/Cargo.toml
	echo "v$(version)" > assets/root/.allium/version.txt
	cargo check
	git add allium-launcher/Cargo.toml
	git add allium-menu/Cargo.toml
	git add alliumd/Cargo.toml
	git add activity-tracker/Cargo.toml
	git add common/Cargo.toml
	git add Cargo.lock
	git add assets/root/.allium/version.txt
	git commit -m "chore: bump version to v$(version)"
	git tag "v$(version)" -a
