FROM debian:bookworm-slim

LABEL org.opencontainers.image.source=https://github.com/cpg314/checkalot
LABEL org.opencontainers.image.licenses=MIT

COPY target-cross/x86_64-unknown-linux-gnu/release/checkalot /usr/bin/checkalot
COPY target-cross/x86_64-unknown-linux-gnu/release/cargo-checkalot /usr/bin/cargo-checkalot
COPY target-cross/x86_64-unknown-linux-gnu/release/checkalot-bundle /usr/bin/checkalot-bundle

CMD ["/usr/bin/checkalot"]
