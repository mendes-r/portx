prog :=portx

release :=--release
target :=release

run:
	cargo fmt
	cargo run

build:
	cargo fmt
	cargo build $(release)

