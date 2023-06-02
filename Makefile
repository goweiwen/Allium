ROOT_DIR := $(shell pwd)
BUILD_DIR := target/arm-unknown-linux-gnueabihf/release
DIST_DIR := dist
RETROARCH := third-party/RetroArch
TOOLCHAIN := mholdg16/miyoomini-toolchain:latest

all: $(BUILD_DIR)/allium static $(RETROARCH)/retroarch_miyoo284 $(RETROARCH)/retroarch_miyoo354
	cp $(BUILD_DIR)/allium $(DIST_DIR)/.allium
	cp $(RETROARCH)/retroarch_miyoo354 $(DIST_DIR)/RetroArch/
	cp $(RETROARCH)/retroarch_miyoo284 $(DIST_DIR)/RetroArch/

clean:
	rm -r $(DIST)

$(BUILD_DIR)/allium:
	cross build --release

static:
	cp -r assets/root/. $(DIST_DIR)

$(RETROARCH)/retroarch_miyoo354:
	docker run --rm -v /$(ROOT_DIR)/third-party:/root/workspace $(TOOLCHAIN) bash -c "source /root/.bashrc; cd RetroArch; make clean all ADD_NETWORKING=1 PACKAGE_NAME=retroarch_miyoo354"

$(RETROARCH)/retroarch_miyoo284:
	docker run --rm -v /$(ROOT_DIR)/third-party:/root/workspace $(TOOLCHAIN) bash -c "source /root/.bashrc; cd RetroArch; make clean all PACKAGE_NAME=retroarch_miyoo284"