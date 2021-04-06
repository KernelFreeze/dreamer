FROM docker.io/rustlang/rust:nightly as rust-builder
WORKDIR /usr/src/dreamer
COPY . .
RUN apt update && apt install -y libopus-dev pkg-config
RUN cargo install --path .

FROM python:3
RUN pip3 install --user ytmusicapi lyrics_extractor
RUN apt update && apt install -y ffmpeg youtube-dl
COPY --from=rust-builder /usr/local/cargo/bin/dreamer /usr/local/bin/dreamer
CMD ["dreamer"]