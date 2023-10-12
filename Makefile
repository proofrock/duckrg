.PHONY: test

build-container:
	docker build -t duckrg .

test-container:
	docker run --name duckrg -d -p "12321:12321" duckrg
	sleep 3
	curl -sX POST -d @transaction.json -H "Content-Type: application/json" localhost:12321/tpch
	docker stop duckrg && docker rm -f duckrg

	#docker run --rm -v $$(pwd)/gh_youplot.sh:/tmp/gh_youplot.sh duckrg /tmp/gh_youplot.sh

clean:
	cargo clean
	rm -rf bin
	- docker image prune -af
	- docker builder prune -af

update:
	cargo update
	cd tests && go get -u
	cd tests && go mod tidy

lint:
	cargo clippy 2> clippy_results.txt

test:
	- pkill duckrg
	make build-debug
	cd tests; go test -v -timeout 5m

test-short:
	- pkill duckrg
	make build-debug
	cd tests; go test -v -timeout 1m -short

build-debug:
	cargo build

build:
	cargo build --release

build-static-nostatic:
	rm -rf bin
	- mkdir bin
	bash -c "RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target `uname -m`-unknown-linux-gnu"
	bash -c "tar czf bin/duckrg-v0.17.1-linux-`uname -m`-static-bundled.tar.gz -C target/`uname -m`-unknown-linux-gnu/release/ duckrg"
	cp Cargo.toml Cargo.toml.orig
	sed 's/^duckdb.*$$/duckdb = { version = "~0", features = [\"serde_json\"] }/' Cargo.toml.orig > Cargo.toml
	bash -c "cargo build --release --target `uname -m`-unknown-linux-gnu"
	bash -c "tar czf bin/duckrg-v0.17.1-linux-`uname -m`-dynamic.tar.gz -C target/`uname -m`-unknown-linux-gnu/release/ duckrg"
	mv Cargo.toml.orig Cargo.toml

build-macos:
	rm -rf bin
	- mkdir bin
	cargo build --release
	tar czf bin/duckrg-v0.17.1-macos-x86_64-bundled.tar.gz -C target/release/ duckrg
	cargo build --release --target aarch64-apple-darwin
	tar czf bin/duckrg-v0.17.1-macos-aarch64-bundled.tar.gz -C target/aarch64-apple-darwin/release/ duckrg
	cp Cargo.toml Cargo.toml.orig
	sed 's/^duckdb.*$$/duckdb = { version = "~0", features = [\"serde_json\"] }/' Cargo.toml.orig > Cargo.toml
	cargo build --release
	tar czf bin/duckrg-v0.17.1-macos-x86_64-dynamic.tar.gz -C target/release/ duckrg
	cargo build --release --target aarch64-apple-darwin
	tar czf bin/duckrg-v0.17.1-macos-aarch64-dynamic.tar.gz -C target/aarch64-apple-darwin/release/ duckrg	
	mv Cargo.toml.orig Cargo.toml

docker:
	docker run --privileged --rm tonistiigi/binfmt --install arm64
	docker buildx build --no-cache --platform linux/amd64 -t germanorizzo/duckrg:v0.17.1-x86_64 --push .
	docker buildx build --no-cache --platform linux/arm64 -t germanorizzo/duckrg:v0.17.1-aarch64 --push .
	- docker manifest rm germanorizzo/duckrg:v0.17.1
	docker manifest create germanorizzo/duckrg:v0.17.1 germanorizzo/duckrg:v0.17.1-x86_64 germanorizzo/duckrg:v0.17.1-aarch64
	docker manifest push germanorizzo/duckrg:v0.17.1
	- docker manifest rm germanorizzo/duckrg:latest
	docker manifest create germanorizzo/duckrg:latest germanorizzo/duckrg:v0.17.1-x86_64 germanorizzo/duckrg:v0.17.1-aarch64
	docker manifest push germanorizzo/duckrg:latest

docker-edge:
	# in Cargo.toml, set 'version = "0.x.999"' where x is the current minor
	docker run --privileged --rm tonistiigi/binfmt --install arm64,arm
	docker buildx build --no-cache --platform linux/amd64 -t germanorizzo/duckrg:edge --push .

docker-zbuild-linux:
	- mkdir bin
	docker run --privileged --rm tonistiigi/binfmt --install arm64,arm
	docker buildx build --no-cache --platform linux/amd64 -f Dockerfile.binaries --target export -t tmp_binaries_build . --output bin
	docker buildx build --no-cache --platform linux/arm64 -f Dockerfile.binaries --target export -t tmp_binaries_build . --output bin
	# Doesn't work. armv7-unknown-linux-gnueabihf must be used. Anyway, for now ARMv7 is out of scope.
	# docker buildx build --no-cache --platform linux/arm/v7 -f Dockerfile.binaries --target export -t tmp_binaries_build . --output bin

