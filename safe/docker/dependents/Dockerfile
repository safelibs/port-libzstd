ARG BASE_IMAGE=ubuntu:24.04
FROM ${BASE_IMAGE}

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /opt/libzstd-matrix

RUN apt-get update && apt-get install -y --no-install-recommends \
    apt \
    apt-utils \
    btrfs-progs \
    build-essential \
    ca-certificates \
    cmake \
    cpio \
    curl \
    debhelper \
    devscripts \
    dh-package-notes \
    dpkg-dev \
    fakeroot \
    file \
    help2man \
    jq \
    libarchive-tools \
    liblz4-dev \
    liblzma-dev \
    libtiff-tools \
    less \
    pkgconf \
    python3 \
    python3-pil \
    qemu-utils \
    rpm \
    rsync \
    squashfs-tools \
    systemd \
    zarchive-tools \
    zlib1g-dev \
    zstd && \
    rm -rf /var/lib/apt/lists/*

COPY . /opt/libzstd-matrix

RUN chmod 0755 \
        /opt/libzstd-matrix/safe/docker/dependents/entrypoint.sh \
        /opt/libzstd-matrix/safe/scripts/check-dependent-compile-compat.sh && \
    pkg_dir=/opt/libzstd-matrix/safe/out/deb/default/packages && \
    shopt -s nullglob && \
    lib_debs=("$pkg_dir"/libzstd1_*.deb) && \
    dev_debs=("$pkg_dir"/libzstd-dev_*.deb) && \
    cli_debs=("$pkg_dir"/zstd_*.deb) && \
    [[ ${#lib_debs[@]} -eq 1 && ${#dev_debs[@]} -eq 1 && ${#cli_debs[@]} -eq 1 ]] && \
    dpkg --unpack "${dev_debs[0]}" "${cli_debs[0]}" "${lib_debs[0]}" && \
    dpkg --configure libzstd1 zstd libzstd-dev

ENTRYPOINT ["/opt/libzstd-matrix/safe/docker/dependents/entrypoint.sh"]
CMD ["all"]
