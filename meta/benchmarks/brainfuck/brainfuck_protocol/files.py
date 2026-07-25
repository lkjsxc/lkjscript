"""Verified acquisition and hashing of protocol inputs."""

import hashlib
import pathlib
import urllib.request

def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def fetch_verified(url: str, destination: pathlib.Path, expected_sha256: str) -> None:
    if destination.exists():
        actual = sha256_file(destination)
        if actual != expected_sha256:
            raise RuntimeError(
                f"cached {destination} has SHA-256 {actual}, expected {expected_sha256}"
            )
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".tmp")
    try:
        with urllib.request.urlopen(url, timeout=60) as response, temporary.open(
            "wb"
        ) as output:
            while block := response.read(1024 * 1024):
                output.write(block)
        actual = sha256_file(temporary)
        if actual != expected_sha256:
            raise RuntimeError(
                f"downloaded {url} has SHA-256 {actual}, expected {expected_sha256}"
            )
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)
