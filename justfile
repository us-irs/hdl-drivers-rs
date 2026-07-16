set unstable
set lists

set shell := ["bash", "-euo", "pipefail", "-c"]

crates := "axi-ad9361 axi-dma axi-uart16550 axi-uartlite"

default:
    @just --list

all: check fmt-check clippy build

check:
    @for crate in {{crates}}; do \
      echo "==> just check ($crate)"; \
      (cd "$crate" && just check); \
    done

fmt:
    @for crate in {{crates}}; do \
      echo "==> just fmt ($crate)"; \
      (cd "$crate" && just fmt); \
    done

fmt-check:
    @for crate in {{crates}}; do \
      echo "==> just check-fmt ($crate)"; \
      (cd "$crate" && just check-fmt); \
    done

clippy:
    @for crate in {{crates}}; do \
      echo "==> just clippy ($crate)"; \
      (cd "$crate" && just clippy); \
    done

build:
    @for crate in {{crates}}; do \
      echo "==> just build ($crate)"; \
      (cd "$crate" && just build); \
    done
