# ----------
#    USER
# ----------
FROM alpine:latest AS user
RUN adduser -S -s /bin/false -D xfer
RUN mkdir /dir

# -----------
#    BUILD
# -----------
FROM rust:1-alpine AS build
WORKDIR /build
RUN apk add --no-cache --update build-base

# Pre-cache dependencies
COPY ["./xfer-server/Cargo.toml", "Cargo.lock", "./"]
RUN mkdir src \
    && echo "// Placeholder" > src/lib.rs \
    && cargo build --release \
    && rm src/lib.rs

# Build
COPY ./xfer-server/src ./src
RUN cargo build --release

# -----------
#   RUNTIME
# -----------
FROM scratch
WORKDIR /opt

COPY --from=build /build/target/release/xfer-server /usr/bin/xfer-server

# Import and switch to non-root user.
COPY --from=user /etc/passwd /etc/passwd
COPY --from=user /bin/false /bin/false
USER xfer
COPY --from=user --chown=xfer /dir /srv/xfer-server

ENV XFER_SERVER_ADDRESS=0.0.0.0:8255
ENV XFER_SERVER_DATA_DIRECTORY=/srv/xfer-server
ENV RUST_LOG=info
EXPOSE 8255

ENTRYPOINT ["/usr/bin/xfer-server"]