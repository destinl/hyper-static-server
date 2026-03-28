# Use the official Rust image as the base image
FROM rust:latest as builder

# Set the working directory
WORKDIR /app

# 配置使用国内镜像源（中国用户）
ENV CARGO_HOME=/usr/local/cargo
RUN echo '[source.crates-io]' > $CARGO_HOME/config.toml && \
    echo 'replace-with = "ustc"' >> $CARGO_HOME/config.toml && \
    echo '[source.ustc]' >> $CARGO_HOME/config.toml && \
    echo 'registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"' >> $CARGO_HOME/config.toml

# Copy the project files
COPY Cargo.toml .
COPY src ./src
COPY benches ./benches

# Build the application
RUN cargo build --release

# Rest of the Dockerfile remains the same...
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false hyperuser

WORKDIR /app

COPY --from=builder /app/target/release/hyper-static-server /app/hyper-static-server
COPY public ./public

RUN chown -R hyperuser:hyperuser /app

USER hyperuser

EXPOSE 3000

CMD ["./hyper-static-server", "-d", "./public", "-h", "0.0.0.0"]