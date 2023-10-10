FROM rust:latest as build

RUN apt-get update -y &&  apt-get install -y \
    musl-tools

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src

COPY . .

RUN sed -i 's/^duckdb.*$/duckdb = { version = "~0", features = ["serde_json"] }/' Cargo.toml

ENV LD_LIBRARY_PATH=/usr/lib

RUN wget -O /tmp/libduckdb.zip \
	"https://github.com/duckdb/duckdb/releases/download/v0.9.0/libduckdb-linux-amd64.zip" && \
	unzip /tmp/libduckdb.zip -d /usr/lib && \
	rm /tmp/libduckdb.zip

RUN RUSTFLAGS='-Clinker=musl-gcc -L /usr/lib' ldconfig && \
	cargo build --release #--target=x86_64-unknown-linux-musl

##########

FROM ubuntu

COPY --from=build /usr/src/target/release/duckrg /usr/local/bin

RUN apt-get update -y && apt-get install -y wget unzip

# duckrg is dynamically linked (?) so may ned libduckdb
ENV LD_LIBRARY_PATH=/usr/lib
RUN wget -O /tmp/libduckdb.zip \
	"https://github.com/duckdb/duckdb/releases/download/v0.9.0/libduckdb-linux-amd64.zip" && \
	unzip /tmp/libduckdb.zip -d /usr/lib && \
	rm /tmp/libduckdb.zip

# adding duckdb cli for testing purposes
RUN wget -O /tmp/duckdbcli.zip \
	"https://github.com/duckdb/duckdb/releases/download/v0.9.0/duckdb_cli-linux-amd64.zip" && \
	unzip /tmp/duckdbcli.zip -d /usr/local/bin && \
	rm /tmp/duckdbcli.zip

# add youplot for visuals

RUN apt-get install -y software-properties-common && \
    apt-add-repository -y ppa:rael-gc/rvm && \
    apt-get update -y && apt-get install -y \
    rvm libssl1.0-dev ruby3.0 rubygems ruby-dev

RUN gem install youplot -v 0.4.5

EXPOSE 12321

# generate example data
RUN mkdir /data && duckdb /data/tpch.db -c "load 'tpch'; call dbgen(sf=0.1);"
VOLUME /data

CMD ["duckrg --db /data/tpch.db"]
