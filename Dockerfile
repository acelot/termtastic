# syntax=docker/dockerfile:1

# ---------- builder ----------
FROM rust:1-alpine AS builder

RUN apk add --no-cache \
    build-base \
    ca-certificates \
    git

WORKDIR /src

ARG APP_VERSION=dev
ENV GIT_REF_TYPE=tag \
    GIT_REF_NAME=${APP_VERSION}

COPY Cargo.toml Cargo.lock build.rs LICENSE ./
COPY src ./src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked \
    && cp target/release/termtastic /termtastic

# ---------- runtime ----------
FROM alpine:3 AS runtime

RUN addgroup -g 1000 termtastic \
    && adduser -D -u 1000 -G termtastic -h /home/termtastic -s /bin/sh termtastic \
    && mkdir -p \
    /home/termtastic/.config/termtastic \
    /home/termtastic/.local/share/termtastic \
    && chown -R 1000:1000 /home/termtastic

COPY --from=builder /termtastic /usr/local/bin/termtastic

USER 1000:1000

ENV HOME=/home/termtastic \
    XDG_CONFIG_HOME=/home/termtastic/.config \
    XDG_DATA_HOME=/home/termtastic/.local/share \
    TERM=xterm-256color

WORKDIR /home/termtastic

VOLUME ["/home/termtastic/.config/termtastic", "/home/termtastic/.local/share/termtastic"]

ENTRYPOINT ["termtastic"]
