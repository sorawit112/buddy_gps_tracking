# Get started with a build env with Rust nightly
FROM rust:1.90.0 AS builder

# Install cargo-binstall, which makes it easier to install other
# cargo extensions like cargo-leptos
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN cp cargo-binstall /usr/local/cargo/bin

# Install required tools
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends clang curl \
    && apt-get install -y --no-install-recommends  \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-leptos
RUN cargo binstall cargo-leptos -y

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

# Make an /app dir, which everything will eventually live in
RUN mkdir -p /app
WORKDIR /app

# Cache dependencies (Copy TOML/Lock FIRST)
COPY Cargo.toml Cargo.lock ./

# Create dummy source files to trigger dependency build without full source code
# This step is only re-run when Cargo.toml/Lock changes, maximizing cache hits
RUN mkdir src/ && echo "fn main(){}" > src/lib.rs; \
    cargo build --target wasm32-unknown-unknown --release; \
    rm -rf src/ && rm -rf target/release/.fingerprint

# Copy the rest of the source code
COPY . .

# Build the app
RUN cargo leptos build --release -vv

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# -- NB: update binary name from "leptos_start" to match your app name in Cargo.toml --
# Copy the server binary to the /app directory
# COPY --from=builder /app/target/release/leptos_start /app/
COPY --from=builder /app/target/release/buddy /app/

# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site

# Copy Cargo.toml if it’s needed at runtime
COPY --from=builder /app/Cargo.toml /app/

# Set any required env variables and
ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 8080

# -- NB: update binary name from "leptos_start" to match your app name in Cargo.toml --
# Run the server
# CMD ["/app/leptos_start"]
CMD ["/app/buddy"]
