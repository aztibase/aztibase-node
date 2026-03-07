FROM rust:1.88-bookworm AS builder

WORKDIR /build
COPY . .

RUN cargo build --release --bin aztibase

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/aztibase /usr/local/bin/aztibase

EXPOSE 9944 9000

ENV AZTIBASE_DATA_DIR=/data
ENV AZTIBASE_RPC_ADDR=0.0.0.0:9944
ENV AZTIBASE_LOG=info

VOLUME ["/data"]

ENTRYPOINT ["aztibase"]
CMD ["--data-dir", "/data", "--rpc-addr", "0.0.0.0:9944", "--metrics"]
