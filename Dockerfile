# Use the official Rust image as a parent image
FROM rust:1.80 AS builder

# Set the working directory in the container
WORKDIR /usr/src/app

# Install build dependencies
RUN apt update && apt install -y \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Copy the current directory contents into the container
COPY . .

# Build the application
RUN cargo build --release

# Stage for creating the non-privileged user
FROM debian:12.6-slim AS user-stage

RUN adduser --uid 10001 --system appuser

# Stage for a smaller final image
FROM scratch

# Copy necessary files from the builder stage
COPY --from=builder /usr/src/app/target/release/inv_sig_helper_rust /app/inv_sig_helper_rust
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Copy passwd file for the non-privileged user from the user-stage
COPY --from=user-stage /etc/passwd /etc/passwd

# Set the working directory
WORKDIR /app

# Expose port 12999
EXPOSE 12999

# Switch to non-privileged user
USER appuser

# Set the entrypoint to the binary name
ENTRYPOINT ["/app/inv_sig_helper_rust"]

# Set default arguments in CMD
CMD ["--tcp", "127.0.0.1:12999"]
