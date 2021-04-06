# syntax=docker/dockerfile:experimental

FROM docker.io/rustlang/rust:nightly as rust-builder
WORKDIR /usr/src/dreamer
COPY . .
RUN apt update && apt install -y libopus-dev pkg-config
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/dreamer/target \
    cargo install --path .

FROM python:3
WORKDIR /dreamer
COPY ./i18n /dreamer/i18n
RUN apt update && apt install -y ffmpeg
RUN wget https://yt-dl.org/downloads/latest/youtube-dl -O /usr/local/bin/youtube-dl && chmod a+rx /usr/local/bin/youtube-dl
RUN pip3 install --user ytmusicapi lyrics_extractor
COPY --from=rust-builder /usr/local/cargo/bin/dreamer /usr/local/bin/dreamer
CMD ["dreamer"]