FROM docker.io/rustlang/rust:nightly-alpine3.12 as rust-builder
WORKDIR /usr/src/dreamer
COPY . .
ENV LIBOPUS_STATIC="true"
RUN apk add --no-cache opus-dev musl-dev pkgconfig
RUN cargo install --path .

FROM python:alpine AS python-builder
RUN apk add --update --no-cache python3-dev libxml2-dev libxslt-dev build-base
RUN pip3 install --user ytmusicapi lyrics_extractor

FROM python:alpine
RUN apk add --no-cache ffmpeg youtube-dl libxml2 libxslt
COPY --from=python-builder /root/.local /root/.local
COPY --from=rust-builder /usr/local/cargo/bin/dreamer /usr/local/bin/dreamer
CMD ["dreamer"]