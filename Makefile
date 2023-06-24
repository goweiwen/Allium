ROOT_DIR := $(shell pwd)
BUILD_DIR := target/arm-unknown-linux-gnueabihf/release
DIST_DIR := dist
RETROARCH := third-party/RetroArch
TOOLCHAIN := mholdg16/miyoomini-toolchain:latest

all: static build package-build package-retroarch

simulator-env:
	mkdir -p assets/simulator
	rsync -ar assets/root/.allium assets/simulator/

simulator-launcher: simulator-env
	RUST_LOG=trace RUST_BACKTRACE=1 ALLIUM_BASE_DIR=assets/simulator/.allium ALLIUM_GAMES_DIR=assets/simulator/Roms cargo run --bin allium-launcher

simulator-menu: simulator-env
	RUST_LOG=trace RUST_BACKTRACE=1 ALLIUM_BASE_DIR=assets/simulator/.allium ALLIUM_GAMES_DIR=assets/simulator/Roms cargo run --bin allium-menu

clean:
	rm -r $(DIST_DIR)

static:
	mkdir -p $(DIST_DIR)
	rsync -a --exclude='.gitkeep' assets/root/. $(DIST_DIR)

build:
	cross build --release

package-build:
	rsync -a $(BUILD_DIR)/alliumd $(DIST_DIR)/.allium
	rsync -a $(BUILD_DIR)/allium-launcher $(DIST_DIR)/.allium
	rsync -a $(BUILD_DIR)/allium-menu $(DIST_DIR)/.allium

retroarch: $(RETROARCH)/retroarch_miyoo283 $(RETROARCH)/retroarch_miyoo354

package-retroarch: retroarch
	rsync -a $(RETROARCH)/retroarch_miyoo354 $(DIST_DIR)/RetroArch/
	rsync -a $(RETROARCH)/retroarch_miyoo283 $(DIST_DIR)/RetroArch/

$(RETROARCH)/retroarch_miyoo354:
	docker run --rm -v /$(ROOT_DIR)/third-party:/root/workspace $(TOOLCHAIN) bash -c "source /root/.bashrc; cd RetroArch; make clean all ADD_NETWORKING=1 PACKAGE_NAME=retroarch_miyoo354"

$(RETROARCH)/retroarch_miyoo283:
	docker run --rm -v /$(ROOT_DIR)/third-party:/root/workspace $(TOOLCHAIN) bash -c "source /root/.bashrc; cd RetroArch; make clean all PACKAGE_NAME=retroarch_miyoo283"

lint:
	cargo fmt
	cargo clippy --fix --allow-dirty --allow-staged --all-targets

bump-version: lint
	sed -i "s/^version = \".*\"/version = \"$(version)\"/" allium-launcher/Cargo.toml
	sed -i "s/^version = \".*\"/version = \"$(version)\"/" allium-menu/Cargo.toml
	sed -i "s/^version = \".*\"/version = \"$(version)\"/" alliumd/Cargo.toml
	sed -i "s/^version = \".*\"/version = \"$(version)\"/" common/Cargo.toml
	cargo check
