# syntax=docker/dockerfile:1

FROM ubuntu:24.04 AS base

ENV DEBIAN_FRONTEND=noninteractive
ENV LANG=C.UTF-8
ENV LC_ALL=C.UTF-8

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      autoconf \
      automake \
      build-essential \
      ca-certificates \
      cmake \
      dbus-x11 \
      extra-cmake-modules \
      file \
      gettext \
      gzip \
      libbtparse-dev \
      libcdio-dev \
      libexempi-dev \
      libkf5archive-dev \
      libkf5cddb-dev \
      libkf5codecs-dev \
      libkf5config-dev \
      libkf5configwidgets-dev \
      libkf5coreaddons-dev \
      libkf5crash-dev \
      libkf5doctools-dev \
      libkf5filemetadata-dev \
      libkf5guiaddons-dev \
      libkf5i18n-dev \
      libkf5iconthemes-dev \
      libkf5itemmodels-dev \
      libkf5jobwidgets-dev \
      libkf5khtml-dev \
      libkf5kio-dev \
      libkf5newstuff-dev \
      libkf5sane-dev \
      libkf5solid-dev \
      libkf5sonnet-dev \
      libkf5textwidgets-dev \
      libkf5wallet-dev \
      libkf5widgetsaddons-dev \
      libkf5xmlgui-dev \
      libksanecore-dev \
      libpcre3-dev \
      libpoppler-qt5-dev \
      libqt5charts5-dev \
      libtag1-dev \
      libtool \
      libxlsxwriter-dev \
      libyaz-dev \
      make \
      ninja-build \
      pkg-config \
      python3 \
      qtbase5-dev \
      qtwebengine5-dev \
      tidy \
      xauth \
      xvfb \
      yaz \
      zlib1g-dev \
 && rm -rf /var/lib/apt/lists/*

FROM base AS with-local-libcsv

COPY target/downstream/image-build/packages/ /tmp/downstream/packages/

RUN set -eux; \
    dpkg -i /tmp/downstream/packages/*.deb; \
    ldconfig

FROM with-local-libcsv AS prepared

COPY downstream/ /opt/downstream/harness/downstream/
COPY target/downstream/install/ /opt/downstream/apps/

RUN set -eux; \
    mkdir -p /work/target/downstream/build /work/target/downstream/sources; \
    if [ -d /opt/downstream/apps/tellico/opt/downstream-support/build ]; then \
      ln -s /opt/downstream/apps/tellico/opt/downstream-support/build /work/target/downstream/build/tellico; \
    fi; \
    if [ -d /opt/downstream/apps/tellico/opt/downstream-support/source ]; then \
      ln -s /opt/downstream/apps/tellico/opt/downstream-support/source /work/target/downstream/sources/tellico; \
    fi

WORKDIR /opt/downstream/harness
