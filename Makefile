TARGET=gh-mozhu

dist: default darwin-x86 darwin-aarch64 linux windows

default:
	cargo build -r
	cp ./target/release/${TARGET} ${TARGET}

linux:
	cargo build -r --target=x86_64-unknown-linux-gnu
	cp ./target/x86_64-unknown-linux-gnu/release/${TARGET} ${TARGET}-linux-amd64

windows:
	cargo build -r --target=x86_64-pc-windows-gnu
	cp ./target/x86_64-pc-windows-gnu/release/${TARGET}.exe ${TARGET}-windows-amd64.exe

darwin-x86:
	SDKROOT=/opt/MacOSX13.3.sdk \
	PATH="$$PATH:~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/" \
	CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=rust-lld \
	cargo build -r --target=x86_64-apple-darwin
	cp ./target/x86_64-apple-darwin/release/${TARGET} ${TARGET}-darwin-amd64

darwin-aarch64:
	SDKROOT=/opt/MacOSX13.3.sdk \
	CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=rust-lld \
	CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=rust-lld \
	PATH="$$PATH:~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/" \
	cargo build -r --target=aarch64-apple-darwin
	cp ./target/aarch64-apple-darwin/release/${TARGET} ${TARGET}-darwin-arm64

drawin-x86: darwin-x86

drawin-aarch64: darwin-aarch64
