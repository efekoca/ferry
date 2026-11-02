ARG PHP=8.4
FROM php:${PHP}-cli

# Debian 9 (PHP 7.0) has no signed archive any more, so that image skips the libraries and
# only builds the extensions that need none; generate.mjs compares extensions present in both.
RUN . /etc/os-release \
    && if grep -q stretch /etc/os-release; then exit 0; fi \
    && case "$VERSION_CODENAME" in buster|bullseye) \
         sed -i -e 's|deb.debian.org|archive.debian.org|g' -e 's|security.debian.org|archive.debian.org|g' /etc/apt/sources.list ;; \
       esac \
    && apt-get update \
    && apt-get install -y --no-install-recommends libpng-dev libicu-dev libzip-dev libpq-dev libldap2-dev libgmp-dev libxml2-dev libxslt1-dev libbz2-dev libtidy-dev unixodbc-dev \
         "$(apt-cache show libenchant-2-dev >/dev/null 2>&1 && echo libenchant-2-dev || echo libenchant-dev)" \
    && rm -rf /var/lib/apt/lists/*

RUN for ext in bcmath bz2 calendar enchant exif gd gettext gmp intl mysqli opcache pcntl pdo_mysql pdo_pgsql pgsql shmop soap sockets sysvmsg sysvsem sysvshm tidy xsl zip; do \
      docker-php-ext-install -j"$(nproc)" "$ext" >/dev/null 2>&1 || echo "skipped $ext"; \
    done \
    && (docker-php-ext-configure ldap --with-libdir="lib/$(gcc -print-multiarch)" >/dev/null 2>&1 && docker-php-ext-install ldap >/dev/null 2>&1 || echo "skipped ldap") \
    && (docker-php-ext-configure pdo_odbc --with-pdo-odbc=unixODBC,/usr >/dev/null 2>&1 && docker-php-ext-install pdo_odbc >/dev/null 2>&1 || echo "skipped pdo_odbc")

COPY dump.php /dump.php
CMD ["php", "-d", "error_reporting=0", "/dump.php"]
