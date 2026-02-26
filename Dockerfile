# Use the official Rust image as a builder stage
FROM rust:1.70 as builder

# Set the working directory
WORKDIR /usr/src/myapp

# Copy the source code 
COPY . .

# Build the application
RUN cargo build --release

# Use the official minimal image for the final stage
FROM debian:buster-slim

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/myapp/target/release/myapp /usr/local/bin/

# Set the command to run the application
CMD ["myapp"]

# Expose the port the app runs on
EXPOSE 8080
