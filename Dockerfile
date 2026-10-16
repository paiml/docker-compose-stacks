FROM rust:1-alpine AS build
WORKDIR /src
RUN apk add --no-cache musl-dev
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release && mkdir /data

FROM scratch
COPY --from=build /src/target/release/stackd /stackd
COPY --from=build --chown=10001:10001 /data /data
USER 10001
ENTRYPOINT ["/stackd"]
