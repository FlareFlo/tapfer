FROM docker.io/rust:1.92 as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY ./src ./src
COPY templates ./templates

RUN cargo build --release

FROM docker.io/archlinux
WORKDIR /usr/src/app
COPY --from=builder /usr/src/app/target/release/tapfer .
COPY ./static ./static

CMD ["./tapfer"]