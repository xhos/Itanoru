FROM rust:slim

LABEL org.opencontainers.image.source="https://github.com/xhos/Itanoru"

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    python3 \
    python3-full \
    python3-pip \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*

# Create and activate virtual environment for gallery-dl
RUN python3 -m venv /opt/venv
ENV PATH="/opt/venv/bin:$PATH"

# Install gallery-dl in virtual environment
RUN pip3 install --no-cache-dir gallery-dl

# Set working directory
WORKDIR /app

# Create data directory with proper permissions
RUN mkdir -p /app/data && chmod 777 /app/data

# Copy the Rust project files
COPY . .

# Build the application
RUN cargo build --release

# Run the boto
CMD ["./target/release/itanoru"]
