# Use the official Alpine-based Rust image as a parent image
FROM rust:1.80-alpine AS builder

# Set the working directory in the container
WORKDIR /usr/src/app

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    openssl-dev \
    openssl-libs-static \
    pkgconfig \
    patch

# Set environment variables for static linking
ENV OPENSSL_STATIC=yes
ENV OPENSSL_DIR=/usr

# Copy the current directory contents into the container
COPY . .

# Build the application
RUN rustup target add x86_64-unknown-linux-musl
RUN RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target x86_64-unknown-linux-musl

# Create a new stage for a smaller final image
FROM scratch

# Copy necessary files from the builder stage
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/inv_sig_helper_rust /app/inv_sig_helper_rust
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Set the working directory
WORKDIR /app

# Expose port 12999
EXPOSE 12999

# Set the entrypoint to the binary name
ENTRYPOINT ["/app/inv_sig_helper_rust"]

# Set default arguments in CMD
CMD ["--tcp", "0.0.0.0:12999"]
