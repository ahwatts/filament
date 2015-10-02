FROM scratch
MAINTAINER ReverbNation DevOps <devops@reverbnation.com>

ADD filament-centos7-chroot.tar.xz /
ADD dist/filament-0.3.0-x86_64-unknown-linux-gnu.tar.gz /
COPY entrypoint.sh /
RUN chmod 0755 /entrypoint.sh

ENV RUST_LOG warn
EXPOSE 7001 7500
ENTRYPOINT [ "/entrypoint.sh" ]
