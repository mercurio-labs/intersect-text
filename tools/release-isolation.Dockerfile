# Build this tool-only image before running consumers with --network none.
# No repository or Cargo cache is copied into the image.
FROM node:22.15.1-bookworm-slim@sha256:ec318fe0dc46b56bcc1ca42a202738aeb4f3e347a7b4dd9f9f1df12ea7aa385a AS node
FROM rust:1.88.0-bookworm@sha256:af306cfa71d987911a781c37b59d7d67d934f49684058f96cf72079c3626bfe0
COPY --from=node /usr/local/bin/node /usr/local/bin/node
RUN apt-get update && apt-get install -y --no-install-recommends python3 \
    && rm -rf /var/lib/apt/lists/* \
    && rustup component add clippy \
    && rustup target add wasm32-unknown-unknown
ENV CARGO_NET_OFFLINE=true
WORKDIR /stage
ENTRYPOINT ["python3", "-B", "/stage/verification/verify_text_stage.py"]
