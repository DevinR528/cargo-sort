FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --verbose --locked --release
RUN strip /app/target/release/cargo-sort

FROM debian:buster-slim AS runtime
WORKDIR app
COPY --from=builder /app/target/release/cargo-sort /usr/local/bin
USER 1000:1000
ENTRYPOINT ["cargo-sort"]
