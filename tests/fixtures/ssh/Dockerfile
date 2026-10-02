# Disposable OpenSSH integration fixture; never a product/release image.
FROM ubuntu@sha256:a853f94d226358a79c740cfc7bce0c289748f3fe3488d921d038ccd752c61b60
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends openssh-server bash zsh fish ca-certificates && rm -rf /var/lib/apt/lists/*
# Password authentication is disabled by the runtime fixture's sshd config.
RUN useradd --create-home --uid 1001 --shell /bin/bash fixture && usermod --password x fixture
