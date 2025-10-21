FROM rust:1.90

WORKDIR /usr/src/whitewater
COPY . .
EXPOSE 8090

RUN cargo install --path .

CMD ["whitewater"]
