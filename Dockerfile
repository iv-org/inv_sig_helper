# Use the official Rust image as a parent image
FROM rust:1.80 AS builder

# Set the working directory in the container
WORKDIR /usr/src/app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    musl-tools \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install OpenSSL for musl
ENV OPENSSL_VERSION=1.1.1u
RUN wget https://www.openssl.org/source/openssl-${OPENSSL_VERSION}.tar.gz \
    && tar zxvf openssl-${OPENSSL_VERSION}.tar.gz \
    && cd openssl-${OPENSSL_VERSION} \
    && ./Configure no-shared no-async --prefix=/usr/local/musl --openssldir=/usr/local/musl linux-x86_64 \
    && make -j$(nproc) \
    && make install \
    && cd .. \
    && rm -rf openssl-${OPENSSL_VERSION}*

# Set OpenSSL directory for musl
ENV OPENSSL_DIR=/usr/local/musl
ENV OPENSSL_INCLUDE_DIR=/usr/local/musl/include
ENV OPENSSL_LIB_DIR=/usr/local/musl/lib

# Copy the current directory contents into the container
COPY . .

# Build the application
RUN rustup target add x86_64-unknown-linux-musl
RUN PKG_CONFIG_ALLOW_CROSS=1 \
    RUSTFLAGS="-C linker=x86_64-linux-gnu-gcc" \
    cargo build --release --target x86_64-unknown-linux-musl

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
