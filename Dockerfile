FROM rust:1.82-bookworm as builder

# Install cargo-binstall to install cargo-leptos
# RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-armv7-unknown-linux-musleabihf.full.tgz
# RUN tar -xvf cargo-binstall-armv7-unknown-linux-musleabihf.full.tgz
# RUN cp cargo-binstall /usr/local/cargo/bin

# Install cargo-leptos
RUN cargo install cargo-leptos

RUN apt update && apt install -y npm
RUN npm install -g sass

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

# Copy over build files
WORKDIR /ironnest
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src
COPY ./migrations ./migrations
COPY ./public ./public
COPY ./style ./style

# Build your application
RUN cargo leptos build --release -vv

# Use the debian bookworm slim image as the base image
FROM debian:bookworm-slim

# Install openssl and update CA certificates
RUN apt update && apt install -y openssl ca-certificates && \
    apt clean && \
    rm -rf /var/lib/apt/lists/*

# Copy the build artifact from the builder stage
COPY --from=builder /ironnest/target/release/iron_nest /app/
# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /ironnest/target/site /app/site
# Copy Cargo.toml if it’s needed at runtime
COPY --from=builder /ironnest/Cargo.toml /app/
WORKDIR /app

# Set any required env variables and
ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:80"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 80

# Run the server
CMD ["/app/iron_nest"]
