SUBMODULE_DIR = submodules/dropbear
BUILD_DIR = build
SRC_DIR = src
PATCH_DIR = patches
BIN_DIR = bin

CONFIGURE_FLAGS ?= --host=arm-linux-gnueabihf --disable-zlib --disable-harden \
	--disable-pam --disable-syslog --disable-shadow --disable-lastlog \
	--disable-utmp --disable-utmpx --disable-wtmp --disable-wtmpx \
	--disable-loginfunc --disable-pututline --disable-pututxline

# libutil static (device lacks libutil.so.1), libc/libcrypt dynamic.
DROPBEAR_LIBS ?= -Wl,-Bstatic -lutil -Wl,-Bdynamic -lcrypt

.PHONY: all build copy-submodule apply-patches assemble clean

all: build

## Submodule management

$(BUILD_DIR)/.has_submodule:
	@if [ -d "$(SUBMODULE_DIR)" ] && [ -z "$$(ls -A $(SUBMODULE_DIR))" ]; then \
		git submodule update --init --recursive; \
	fi
	mkdir -p $(BUILD_DIR)
	cp -r $(SUBMODULE_DIR)/* $(BUILD_DIR)/
	@touch $(BUILD_DIR)/.has_submodule

copy-submodule: $(BUILD_DIR)/.has_submodule

## Patch management

$(BUILD_DIR)/.is_patched: $(BUILD_DIR)/.has_submodule
	@for patch in $$(ls $(PATCH_DIR)/*.patch 2>/dev/null | sort); do \
		echo "Applying $$patch"; \
		patch -d $(BUILD_DIR) -p1 < $$patch; \
	done
	@touch $(BUILD_DIR)/.is_patched

apply-patches: $(BUILD_DIR)/.is_patched

## Source management

$(BUILD_DIR)/.is_assembled: $(BUILD_DIR)/.is_patched
	cp -r $(SRC_DIR)/* $(BUILD_DIR)/
	@touch $(BUILD_DIR)/.is_assembled

assemble: $(BUILD_DIR)/.is_assembled

## Build targets

$(BIN_DIR)/dropbear: $(BUILD_DIR)/.is_assembled
	cd $(BUILD_DIR) && sh configure $(CONFIGURE_FLAGS)
	cd $(BUILD_DIR) && $(MAKE) PROGRAMS=dropbear LIBS="$(DROPBEAR_LIBS)"
	mkdir -p $(BIN_DIR)
	cp $(BUILD_DIR)/dropbear $(BIN_DIR)/dropbear

build: $(BIN_DIR)/dropbear

## Clean everything

clean:
	rm -rf $(BUILD_DIR) $(BIN_DIR) || true
