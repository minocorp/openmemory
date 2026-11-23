FROM postgres:17 AS build
ARG PG_BIGM_VERSION=1.2-20261118
ARG PG_BIGM_SHA256=7bd01796351ade57f8faad1587be27cfaa0a5785f3d6cadc36631a4abaa0ec09
RUN apt-get update \
 && apt-get install -y --no-install-recommends build-essential ca-certificates curl libicu-dev postgresql-server-dev-17
WORKDIR /src
RUN curl -fsSL -o pg_bigm.tar.gz "https://github.com/pgbigm/pg_bigm/archive/refs/tags/v${PG_BIGM_VERSION}.tar.gz" \
 && echo "${PG_BIGM_SHA256}  pg_bigm.tar.gz" | sha256sum -c - \
 && tar xzf pg_bigm.tar.gz --strip-components=1 \
 && make USE_PGXS=1 \
 && make USE_PGXS=1 install DESTDIR=/out

FROM postgres:17
COPY --from=build /out/usr/ /usr/
RUN apt-get update \
 && apt-get install -y --no-install-recommends pgbackrest restic jq \
 && rm -rf /var/lib/apt/lists/* \
 && install -d -o postgres -g postgres /backup /etc/pgbackrest /var/log/pgbackrest
ENV PGBACKREST_STANZA=memory
COPY --chmod=755 bin/ /usr/local/bin/
ENTRYPOINT ["memory-entrypoint"]
CMD ["postgres"]
