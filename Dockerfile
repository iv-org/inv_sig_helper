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

# Determine the target architecture and build the application
RUN RUST_TARGET=$(rustc -vV | sed -n 's/host: //p') && \
    rustup target add $RUST_TARGET && \
    RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target $RUST_TARGET

# Create a new stage for a smaller final image
FROM scratch

# Copy necessary files from the builder stage, using the correct architecture path
COPY --from=builder /usr/src/app/target/*/release/inv_sig_helper_rust /app/inv_sig_helper_rust
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Set the working directory
WORKDIR /app

# Expose port 12999
EXPOSE 12999

# Set the entrypoint to the binary name
ENTRYPOINT ["/app/inv_sig_helper_rust"]

# Set default arguments in CMD
CMD ["--tcp", "127.0.0.1:12999"]
