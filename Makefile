.PHONY: build run deps clean test

LIBGIT2_PREFIX ?= $(PWD)/.deps/libgit2
PKG_CONFIG_PATH := $(LIBGIT2_PREFIX)/lib/pkgconfig

export CGO_CFLAGS := -I$(LIBGIT2_PREFIX)/include
export CGO_LDFLAGS := -L$(LIBGIT2_PREFIX)/lib -lgit2 -Wl,-rpath,$(LIBGIT2_PREFIX)/lib
export PKG_CONFIG_PATH

deps:
	@echo "Building libgit2 1.5.2..."
	@mkdir -p .deps
	@if [ ! -d "/tmp/libgit2-1.5.2" ]; then \
		curl -sL https://github.com/libgit2/libgit2/archive/refs/tags/v1.5.2.tar.gz | tar xz -C /tmp; \
	fi
	@cd /tmp/libgit2-1.5.2 && cmake -Bbuild -H. \
		-DBUILD_TESTS=OFF \
		-DCMAKE_INSTALL_PREFIX=$(LIBGIT2_PREFIX) \
		-DUSE_HTTPS=SecureTransport \
		-DUSE_SSH=OFF \
		> /dev/null 2>&1
	@cd /tmp/libgit2-1.5.2 && cmake --build build --parallel $$(sysctl -n hw.ncpu) > /dev/null 2>&1
	@cd /tmp/libgit2-1.5.2 && cmake --build build --target install > /dev/null 2>&1
	@echo "libgit2 installed to $(LIBGIT2_PREFIX)"

build: deps
	go build -o cogit .

run: deps
	go run .

test: deps
	go test ./...

clean:
	rm -f cogit
	rm -rf .deps
