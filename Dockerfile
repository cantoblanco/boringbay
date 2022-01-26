FROM ubuntu:latest
ENV TZ="Asia/Shanghai"

RUN export DEBIAN_FRONTEND="noninteractive" && apt update && apt install -y ca-certificates tzdata \
    libsqlite3-dev && \
    update-ca-certificates && \
    ln -fs /usr/share/zoneinfo/$TZ /etc/localtime && \
    dpkg-reconfigure tzdata

WORKDIR /webapp
COPY ./target/release/naive ./naive
COPY ./migrations ./migrations
COPY ./resources ./resources
COPY ./templates ./templates

VOLUME ["/webapp/data"]
CMD ["/webapp/naive"]
