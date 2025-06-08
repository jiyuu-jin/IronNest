FROM rust:1.86-bookworm AS chef
RUN cargo install cargo-chef --locked

FROM chef AS planner
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./crates ./crates
COPY ./src ./src
COPY ./migrations ./migrations
COPY ./public ./public
COPY ./style ./style
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

# Install cargo-binstall to install cargo-leptos
# RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-armv7-unknown-linux-musleabihf.full.tgz
# RUN tar -xvf cargo-binstall-armv7-unknown-linux-musleabihf.full.tgz
# RUN cp cargo-binstall /usr/local/cargo/bin

RUN cargo install cargo-leptos@0.2.35 --locked

RUN apt update && apt install -y npm
RUN npm install -g sass

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

COPY --from=planner recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy over build files
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./crates ./crates
COPY ./src ./src
COPY ./migrations ./migrations
COPY ./public ./public
COPY ./style ./style

# Build your application
RUN cargo leptos build --release -vv

# Use the debian bookworm slim image as the base image
FROM debian:bookworm-slim AS runtime

# Install openssl and update CA certificates
RUN apt update && apt install -y openssl ca-certificates && \
    apt clean && \
    rm -rf /var/lib/apt/lists/*

# Copy the build artifact from the builder stage
COPY --from=builder target/release/iron_nest /app/
# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder target/site /app/site
# Copy Cargo.toml if it’s needed at runtime
COPY --from=builder Cargo.toml /app/
WORKDIR /app

# Set any required env variables and
ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:80"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 80

# Run the server
CMD ["/app/iron_nest"]
