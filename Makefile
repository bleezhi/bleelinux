.PHONY: check build release test iso clean

check:
	cargo check

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

iso: release
	./iso/mkiso.sh

clean:
	cargo clean
	rm -rf /tmp/opencode/bleelinux-root /tmp/opencode/initramfs.cpio.zst
