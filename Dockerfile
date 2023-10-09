FROM alpine:latest as build

RUN apk add --no-cache rust cargo sed

COPY . /build
WORKDIR /build

RUN cp Cargo.toml Cargo.toml.orig
RUN sed 's/^duckdb.*$/duckdb = { version = "~0", features = ["serde_json"] }/' Cargo.toml.orig > Cargo.toml

RUN ["cargo", "build", "--release"]

# Now copy it into our base image.
FROM alpine:latest

COPY --from=build /build/target/release/duckrg /

# TODO why libgcc?
RUN apk add --no-cache libgcc

EXPOSE 12321
VOLUME /data

CMD ["/duckrg"]
