BINARY = gopher-cli
DIST = dist

.PHONY: build build-darwin-arm64 build-darwin-x86 build-linux-arm64 build-linux-x86 build-all clean

build: build-darwin-arm64

build-darwin-arm64:
	cargo build --release -p $(BINARY) --target aarch64-apple-darwin
	@mkdir -p $(DIST)
	cp target/aarch64-apple-darwin/release/$(BINARY) $(DIST)/$(BINARY)-darwin-arm64

build-darwin-x86:
	cargo build --release -p $(BINARY) --target x86_64-apple-darwin
	@mkdir -p $(DIST)
	cp target/x86_64-apple-darwin/release/$(BINARY) $(DIST)/$(BINARY)-darwin-x86

build-linux-arm64:
	cross build --release -p $(BINARY) --target aarch64-unknown-linux-gnu
	@mkdir -p $(DIST)
	cp target/aarch64-unknown-linux-gnu/release/$(BINARY) $(DIST)/$(BINARY)-linux-arm64

build-linux-x86:
	cross build --release -p $(BINARY) --target x86_64-unknown-linux-gnu
	@mkdir -p $(DIST)
	cp target/x86_64-unknown-linux-gnu/release/$(BINARY) $(DIST)/$(BINARY)-linux-x86

build-all: build-darwin-arm64 build-darwin-x86 build-linux-arm64 build-linux-x86

clean:
	rm -rf $(DIST)
