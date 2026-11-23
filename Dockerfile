FROM node:22-slim AS web
RUN corepack enable
WORKDIR /web
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile
COPY web ./
RUN pnpm build

FROM rust:1.100-slim-trixie AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
COPY .sqlx .sqlx
COPY --from=web /web/dist web/dist
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked -p memory-server

FROM debian:trixie-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates poppler-utils \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --no-create-home memory \
 && install -d -o memory -g memory /blobs
COPY --from=build /src/target/release/memory-server /usr/local/bin/memory-server
USER memory
ENV MEMORY_BLOB_DIR=/blobs
EXPOSE 8080
ENTRYPOINT ["memory-server"]
