FROM ubuntu:24.04
ARG TARGETPLATFORM
ENV TZ="Asia/Shanghai"

RUN export DEBIAN_FRONTEND="noninteractive" && apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates tzdata && \
    update-ca-certificates && \
    ln -fs /usr/share/zoneinfo/$TZ /etc/localtime && \
    dpkg-reconfigure --frontend noninteractive tzdata && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /webapp
COPY ./artifact/$TARGETPLATFORM/naive ./naive
RUN chmod +x ./naive
COPY ./migrations ./migrations
COPY ./resources ./resources
COPY ./templates ./templates

VOLUME ["/webapp/data"]
CMD ["/webapp/naive"]
