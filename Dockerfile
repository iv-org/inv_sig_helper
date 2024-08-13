# Use the official Rust image as a parent image with a specific version
FROM rust:1.80 AS builder

# Set the working directory in the container
WORKDIR /usr/src/app

# Copy the current directory contents into the container
COPY . .

# Build the application
RUN cargo build --release

# Start a new stage for a smaller final image
FROM ubuntu:24.10

# Set the working directory in the container
WORKDIR /usr/local/bin

# Install OpenSSL and ca-certificates, then clean up in a single RUN command
RUN apt-get update && \
    apt-get install -y --no-install-recommends openssl ca-certificates && \
    rm -rf /var/lib/apt/lists/* && \
    groupadd -g 10001 appuser && \
    useradd -u 10000 -g appuser appuser

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/inv_sig_helper_rust .

# Change ownership of the binary to the non-root user
RUN chown appuser:appuser inv_sig_helper_rust

# Switch to non-root user
USER appuser:appuser

# Expose port 12999
EXPOSE 12999

# Set the entrypoint to the binary name
ENTRYPOINT ["./inv_sig_helper_rust"]

# Set default arguments in CMD
# CMD ["--tcp", "0.0.0.0:12999"]

