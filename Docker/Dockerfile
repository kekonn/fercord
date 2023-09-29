ARG RUST_VERSION=1.72.1
ARG APP_NAME=fercord

FROM rust:${RUST_VERSION}-slim-bullseye AS builder
ARG APP_NAME
RUN USER=root cargo new --bin ${APP_NAME}
WORKDIR /${APP_NAME}
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --release --locked
RUN rm src/*.rs
ADD . ./
RUN rm ./target/release/deps/${APP_NAME}*
RUN cargo build --release --locked

FROM debian:bullseye-slim AS runner
RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

ARG APP_NAME
ARG UID=1001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    appuser

RUN mkdir /config /app /data
COPY ./Docker/config.toml /config/config.toml
RUN chown -R ${UID}:${UID} /config /app /data

ENV TZ=Etc/UTC \
    CONFIG=/config/config.toml \
    RUST_BACKTRACE=0 \
    RUST_LOG="info,sqlx::query=warn"

WORKDIR /app
COPY --from=builder /${APP_NAME}/target/release/${APP_NAME} .

RUN chown -R $APP_USER:$APP_USER /app

USER appuser

VOLUME [ "/data" ]

CMD ["/app/fercord"]