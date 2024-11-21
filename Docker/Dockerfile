ARG APP_NAME=fercord
ARG EXEC_NAME=fercord_bot
ARG VARIANT="bookworm"

FROM rust:1-slim-${VARIANT} AS builder
ARG APP_NAME
ARG EXEC_NAME
RUN USER=root cargo new --bin ${APP_NAME}
WORKDIR /${APP_NAME}
COPY . .

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo build --bin ${EXEC_NAME} --release --locked
RUN rm -rf fercord_storage/**.rs ${EXEC_NAME}/**.rs fercord_common/**.rs
ADD . ./
RUN rm ./target/release/deps/${APP_NAME}*
RUN cargo build --bin ${EXEC_NAME} --release --locked

FROM debian:${VARIANT}-slim AS runner
RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

ARG APP_NAME
ARG UID=1001
RUN adduser \
    --disabled-password \
    --gecos "" \
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
COPY --from=builder /${APP_NAME}/target/release/${EXEC_NAME} ./${APP_NAME}

RUN chown -R $APP_USER:$APP_USER /app

USER appuser

VOLUME [ "/data" ]

CMD ["/app/fercord"]

HEALTHCHECK --timeout=5s CMD /app/fercord healthcheck