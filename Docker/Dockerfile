FROM rust:1.71 as builder

RUN USER=root cargo new --bin fercord
WORKDIR ./fercord
COPY ./Cargo.toml ./Cargo.toml
RUN cargo build --release
RUN rm src/*.rs

ADD . ./

RUN rm ./target/release/deps/fercord*
RUN cargo build --release

FROM debian:buster-slim AS runner
ARG APP=/usr/src/app

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*
    
RUN mkdir /config
COPY ./Docker/config.toml /config/config.toml

ENV TZ=Etc/UTC \
    APP_USER=appuser \
    RUST_BACKTRACE=0 \
    RUST_LOG="info,sqlx::query=warn"

RUN groupadd $APP_USER \
    && useradd -g $APP_USER $APP_USER \
    && mkdir -p ${APP}

COPY --from=builder /fercord/target/release/fercord ${APP}/fercord

RUN chown -R $APP_USER:$APP_USER ${APP}

USER $APP_USER
WORKDIR ${APP}

CMD ["./fercord"]