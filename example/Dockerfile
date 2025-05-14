FROM debian:bullseye-slim AS stage
WORKDIR /dl
RUN cd /dl && apt update && \
    apt install wget -y && \
    wget https://github.com/suirad/spacegate/releases/download/v1.1/spacegate-proxy-x86_64-linux-musl.tar.gz && \
    tar xf spacegate-proxy-x86_64-linux-musl.tar.gz
RUN ls

FROM gcr.io/distroless/static AS runner
COPY --from=stage /dl/spacegate-proxy /bin/spacegate-proxy
CMD ["/bin/spacegate-proxy"]
