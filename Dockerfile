FROM docker.io/rustlang/rust:nightly-alpine3.12 as builder
WORKDIR /usr/src/dreamer
COPY . .
ENV LIBOPUS_STATIC="true"
RUN apk add --no-cache opus-dev musl-dev pkgconfig
RUN cargo install --path .

FROM alpine:3.7
RUN apk add --no-cache ffmpeg youtube-dl
COPY --from=builder /usr/local/cargo/bin/dreamer /usr/local/bin/dreamer
CMD ["dreamer"]