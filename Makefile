ROOT_DIR := $(shell pwd)
BUILD_DIR := target/arm-unknown-linux-gnueabihf/release
DIST_DIR := dist
RETROARCH := third-party/RetroArch
TOOLCHAIN := mholdg16/miyoomini-toolchain:latest

all: $(BUILD_DIR)/allium static $(RETROARCH)/retroarch_miyoo284 $(RETROARCH)/retroarch_miyoo354
	rsync -a $(BUILD_DIR)/allium $(DIST_DIR)/.allium
	rsync -a $(RETROARCH)/retroarch_miyoo354 $(DIST_DIR)/RetroArch/
	rsync -a $(RETROARCH)/retroarch_miyoo284 $(DIST_DIR)/RetroArch/

clean:
	rm -r $(DIST_DIR)

$(BUILD_DIR)/allium:
	cross build --release

static:
	rsync -a --exclude='.gitkeep' assets/root/. $(DIST_DIR)

$(RETROARCH)/retroarch_miyoo354:
	docker run --rm -v /$(ROOT_DIR)/third-party:/root/workspace $(TOOLCHAIN) bash -c "source /root/.bashrc; cd RetroArch; make clean all ADD_NETWORKING=1 PACKAGE_NAME=retroarch_miyoo354"

$(RETROARCH)/retroarch_miyoo284:
	docker run --rm -v /$(ROOT_DIR)/third-party:/root/workspace $(TOOLCHAIN) bash -c "source /root/.bashrc; cd RetroArch; make clean all PACKAGE_NAME=retroarch_miyoo284"
