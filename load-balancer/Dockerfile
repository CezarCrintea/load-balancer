FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
COPY --from=environment . /app/external_crates/environment
RUN sed -i 's|path = "../environment"|path = "./external_crates/environment"|' /app/Cargo.toml
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
COPY --from=environment . /app/external_crates/environment
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN sed -i 's|path = "../environment"|path = "./external_crates/environment"|' /app/Cargo.toml
RUN cargo build --release --target x86_64-unknown-linux-musl --bin load-balancer

FROM alpine AS runtime
RUN addgroup -S myuser && adduser -S myuser -G myuser
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/load-balancer /usr/local/bin/
USER myuser
CMD ["/usr/local/bin/load-balancer"]