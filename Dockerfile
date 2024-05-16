# Build the image using
#
#    docker build -t full-text-rs .
#
# (this might take a few minutes).
# To run the program in standalone, execute
#
#    docker run --rm full-text-rs make-fulltext <url>
#
# For a service to be reachable on the (host system) port 8080, run
#
#    docker run --rm -p 127.0.0.1:8080:3000 full-text-rs
#
# You can specify a custom config by mounting the corresponding folder to
# /etc/full-text-rs (e.g., -v /path/to/configdir:/etc/full-text-rs:ro)


FROM rust:1.78-alpine3.19 as builder
WORKDIR /build
COPY . .
RUN apk add --no-cache git musl-dev openssl-dev openssl-libs-static libxml2-dev libxml2-static xz-static zlib-static \
 && cargo rustc -r -- -C link-arg=-lz -C link-arg=-llzma \
 && sed -i 's/"127\.0\.0\.1:/"0.0.0.0:/' example/config.toml \
 && git clone https://github.com/fivefilters/ftr-site-config /build/ftr-site-config

FROM alpine:3.19
LABEL org.label-schema.schema-version="1.0"
LABEL org.label-schema.name="full-text-rs"
LABEL org.label-schema.description="Enhance your RSS/atom feeds by transforming them to full text feeds"
LABEL org.label-schema.vcs-url="https://github.com/noctux/full-text-rs/"
WORKDIR /opt
COPY --from=builder /build/target/release/full-text-rs /usr/local/bin/full-text-rs
COPY --from=builder /build/example/config.toml /etc/full-text-rs/config.toml
COPY --from=builder /build/ftr-site-config /opt/ftr-site-config
ENTRYPOINT [ "/usr/local/bin/full-text-rs", "--config", "/etc/full-text-rs/config.toml" ]
CMD [ "serve" ]
