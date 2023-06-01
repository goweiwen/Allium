BUILD = target/arm-unknown-linux-gnueabihf/release/allium
DIST = dist
STATIC = dist/.tmp_update/updater

all: $(STATIC) $(BUILD)
	cp $(BUILD) $(DIST)/.allium

clean:
	rm -r $(DIST)

$(BUILD):
	cross build --release

$(STATIC):
	cp -r assets/root/. $(DIST)