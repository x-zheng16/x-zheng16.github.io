FROM rust:1-bookworm AS homepage-build
WORKDIR /build/cli
COPY cli/Cargo.toml cli/Cargo.lock ./
COPY cli/src ./src
RUN cargo build --release --locked --bin homepage

FROM homepage-build AS homepage-test
COPY cli/tests ./tests
COPY bin/deploy bin/entry_point.sh /build/bin/
RUN cargo test --locked --features e2e --test e2e -- --test-threads=1

FROM ruby:latest
ENV DEBIAN_FRONTEND=noninteractive

LABEL MAINTAINER="Amir Pourmand"

RUN apt-get update -y && apt-get install -y --no-install-recommends \
    locales \
    imagemagick \
    build-essential \
    zlib1g-dev \
    python3-pip \
    inotify-tools git && \
    apt-get clean && rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/* && \
    pip install nbconvert --break-system-packages

RUN sed -i '/en_US.UTF-8/s/^# //g' /etc/locale.gen && \
    locale-gen

ENV LANG=en_US.UTF-8 \
    LANGUAGE=en_US:en \
    LC_ALL=en_US.UTF-8 \
    JEKYLL_ENV=production \
    HOMEPAGE_BIN=/usr/local/bin/homepage

RUN mkdir /srv/jekyll

ADD Gemfile.lock /srv/jekyll
ADD Gemfile /srv/jekyll

WORKDIR /srv/jekyll

RUN gem install jekyll bundler
RUN bundle install --no-cache
EXPOSE 8080

COPY --from=homepage-build /build/cli/target/release/homepage /usr/local/bin/homepage
COPY bin/entry_point.sh /usr/local/bin/entry_point.sh

CMD ["/usr/local/bin/entry_point.sh"]
