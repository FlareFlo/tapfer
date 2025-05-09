FROM docker.io/rust:1.86 as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY ./src ./src
COPY templates ./templates

RUN cargo build --release

FROM docker.io/archlinux
WORKDIR /usr/src/app
COPY --from=builder /usr/src/app/target/release/tapfer .
COPY ./graphics ./graphics

CMD ["./tapfer"]