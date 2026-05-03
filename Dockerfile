FROM docker.io/rust:1.92 as builder

RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./
COPY ./src ./src
COPY templates ./templates
COPY tapfer_crypt ./tapfer_crypt

RUN cargo build --release
RUN cd tapfer_crypt && wasm-pack build --target web --release

FROM docker.io/archlinux
WORKDIR /usr/src/app
COPY --from=builder /usr/src/app/target/release/tapfer .
COPY ./static ./static
COPY --from=builder /usr/src/app/tapfer_crypt/pkg ./tapfer_crypt/pkg

CMD ["./tapfer"]