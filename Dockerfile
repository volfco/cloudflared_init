FROM ekidd/rust-musl-builder as builder

COPY --chown=rust:rust . .
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM cloudflare/cloudflared:2021.11.0
COPY --from=builder /home/rust/src/target/x86_64-unknown-linux-musl/release/cloudflared-init /usr/local/bin/cloudflared-init
CMD ["cloudflared-init"]