# Pensieve on Intel Mac (x86_64)

The standard Pensieve binary is built for Apple Silicon (`aarch64-apple-darwin`).
Intel Macs (`x86_64-apple-darwin`) are not supported natively because the ONNX
Runtime dependency (`ort` via `fastembed`) dropped prebuilt Intel Mac binaries.

This guide sets up Pensieve on Intel Mac using the Linux x86_64 binary running
inside a [Podman](https://podman.io/) container, with a transparent wrapper
script so the `pensieve` command works exactly as documented.

---

## Prerequisites

- macOS on Intel (x86_64)
- [Homebrew](https://brew.sh)
- ~500 MB disk space (Podman VM + model)

---

## Installation

### Step 1 — Install Podman

```bash
brew install podman
podman machine init
podman machine start
```

Podman needs a Linux VM to run containers. This is a one-time setup.

### Step 2 — Download the Linux binary

```bash
curl -fsSL -L \
  https://github.com/rigogsilva/pensieve/releases/latest/download/pensieve-x86_64-unknown-linux-gnu \
  -o ~/pensieve-linux
chmod +x ~/pensieve-linux
```

### Step 3 — Download the embedding model

The ONNX embedding model (`Xenova/bge-small-en-v1.5`) must be downloaded once
and baked into the container image so it doesn't download on every run.

```bash
pip3 install huggingface_hub
python3 -c "
from huggingface_hub import snapshot_download
snapshot_download(repo_id='Xenova/bge-small-en-v1.5', cache_dir='/tmp/pensieve-model')
print('Model downloaded.')
"
```

### Step 4 — Build the container image

Create a `Dockerfile`:

```bash
mkdir -p /tmp/pensieve-docker
cp ~/pensieve-linux /tmp/pensieve-docker/pensieve
cp -r /tmp/pensieve-model /tmp/pensieve-docker/fastembed-cache

cat > /tmp/pensieve-docker/Dockerfile << 'EOF'
FROM ubuntu:24.04
COPY pensieve /usr/local/bin/pensieve
RUN chmod +x /usr/local/bin/pensieve
COPY fastembed-cache /.fastembed_cache
WORKDIR /
ENTRYPOINT ["/usr/local/bin/pensieve"]
EOF
```

Build it:

```bash
cd /tmp/pensieve-docker && podman build -t pensieve:latest .
```

### Step 5 — Create the wrapper script

The wrapper script transparently runs pensieve in the container, mounting your
memory directory and handling the SQLite index (which can't be accessed directly
from iCloud inside a container).

```bash
sudo tee /usr/local/bin/pensieve > /dev/null << 'EOF'
#!/bin/zsh
MEMORY_DIR="/Users/$(whoami)/Library/Mobile Documents/com~apple~CloudDocs/Documents/pensieve"
SQLITE_SRC="${MEMORY_DIR}/index.sqlite"

# Local writable dir for sqlite (SQLite needs to create WAL/journal files here)
SQLITE_DIR="${HOME}/.pensieve-sqlite"
mkdir -p "$SQLITE_DIR"
LOCAL_SQLITE="${SQLITE_DIR}/index.sqlite"

# Sync sqlite from iCloud before run
[[ -f "$SQLITE_SRC" ]] && cp "$SQLITE_SRC" "$LOCAL_SQLITE"

cleanup() {
  [[ -f "$LOCAL_SQLITE" ]] && cp "$LOCAL_SQLITE" "$SQLITE_SRC"
  rm -f "${LOCAL_SQLITE}-wal" "${LOCAL_SQLITE}-shm" "${LOCAL_SQLITE}-journal"
}
trap cleanup EXIT

EXTRA_MOUNTS=()
if [[ "$1" == "setup" ]]; then
  EXTRA_MOUNTS=(-v "${HOME}/.claude:${HOME}/.claude:z")
fi

podman run --rm -i \
  -e "HOME=${HOME}" \
  -v "${SQLITE_DIR}:/sqlite:z" \
  -v "${MEMORY_DIR}/global:/sqlite/global:z" \
  -v "${MEMORY_DIR}/projects:/sqlite/projects:z" \
  -v "${MEMORY_DIR}/sessions:/sqlite/sessions:z" \
  "${EXTRA_MOUNTS[@]}" \
  localhost/pensieve:latest \
  --memory-dir /sqlite "$@"
EOF

sudo chmod +x /usr/local/bin/pensieve
```

> **Note:** If you store your memory directory somewhere other than iCloud
> Drive, replace the `MEMORY_DIR` path with your actual memory directory.

### Step 6 — Configure and set up agents

Point pensieve at your memory directory and run setup:

```bash
# Configure memory directory (adjust path if not using iCloud)
pensieve configure --memory-dir "/Users/$(whoami)/Library/Mobile Documents/com~apple~CloudDocs/Documents/pensieve"

# Install skills for your AI agents
pensieve setup

# Verify it works
pensieve version
pensieve recall "test"
```

Then start a new agent session and say: **"set up pensieve"**

---

## Updating Pensieve

Because the binary is baked into a container image, updating requires rebuilding
the image with the new binary.

```bash
# Download the new binary
curl -fsSL -L \
  https://github.com/rigogsilva/pensieve/releases/latest/download/pensieve-x86_64-unknown-linux-gnu \
  -o ~/pensieve-linux
chmod +x ~/pensieve-linux

# Rebuild the image (model is already cached, no re-download needed)
cp ~/pensieve-linux /tmp/pensieve-docker/pensieve
cd /tmp/pensieve-docker && podman build -t pensieve:latest .
```

Or use this one-liner update script:

```bash
curl -fsSL -L \
  https://github.com/rigogsilva/pensieve/releases/latest/download/pensieve-x86_64-unknown-linux-gnu \
  -o ~/pensieve-linux \
  && chmod +x ~/pensieve-linux \
  && cp ~/pensieve-linux /tmp/pensieve-docker/pensieve \
  && cd /tmp/pensieve-docker \
  && podman build -t pensieve:latest . \
  && echo "Pensieve updated."
```

> **Why not `pensieve update`?** The built-in `pensieve update` command replaces
> the binary at its path — but on Intel Mac, `/usr/local/bin/pensieve` is the
> wrapper script, not the binary. The commands above perform the equivalent
> update for the containerized setup.

---

## How it works

- The **Linux x86_64 binary** runs inside an Ubuntu 24.04 container where ONNX
  Runtime is fully supported
- The **embedding model** (`Xenova/bge-small-en-v1.5`) is baked into the image
  so vector search works without a network call on each run
- Your **memory files** (markdown) are mounted read-write from their source
  location (iCloud or otherwise)
- The **SQLite index** is copied to a local directory before each run (to avoid
  iCloud file locking issues) and synced back on exit
- The **wrapper script** makes all of this transparent — `pensieve recall "foo"`
  works exactly as documented

---

## Troubleshooting

**`podman machine` not running**

```bash
podman machine start
```

**Memories not showing up (iCloud not synced)**

Force local download of all memory files:

```bash
find "/Users/$(whoami)/Library/Mobile Documents/com~apple~CloudDocs/Documents/pensieve" \
  -name "*.md" -exec cat {} \; > /dev/null
```

Then enable "Keep Downloaded" in iCloud Drive settings for the pensieve folder
to prevent files from being evicted.

**Permission denied on memory files**

```bash
chmod -R a+rX "/Users/$(whoami)/Library/Mobile Documents/com~apple~CloudDocs/Documents/pensieve/"
```

**SQLite malformed / corrupted**

If the index gets corrupted (e.g. from concurrent runs), delete it and rebuild:

```bash
rm -f "${HOME}/.pensieve-sqlite/index.sqlite"
rm -f "/Users/$(whoami)/Library/Mobile Documents/com~apple~CloudDocs/Documents/pensieve/index.sqlite"
pensieve reindex
```
