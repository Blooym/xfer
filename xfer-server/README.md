# xfer-server

Server to facilitate transfers between xfer clients.

## Setup

### Docker

1. Copy [compose.yml](./compose.yml) to a local file named `compose.yml` or add the
   service to your existing stack and fill in the environment variables.
   Information about configuration options can be found in the
   [configuration](#configuration) section.

2. Start the stack

```
docker compose up -d
```

### Manual

1. Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed and
   in your `$PATH`.
2. Install the project binary

```
cargo install --git https://github.com/Blooym/xfer.git xfer-server
```

3. Set configuration values as necessary.
   Information about configuration options can be found in the
   [configuration](#configuration) section.

```
xfer-server
```

## Configuration

The xfer server is configured via command-line flags or environment variables and has full support for loading from `.env` files. Below is a list of all supported configuration options. You can also run `xfer-server --help` to get an up-to-date usage information (including default values).

| Name                  | Description                                                                                                                                                                   | Flag                      | Env                             | Default                         |
| --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------- | ------------------------------- | ------------------------------- |
| Address               | The internet socket address that the server should be ran on.                                                                                                                 | `--address`               | `XFER_SERVER_ADDRESS`           | `127.0.0.1:8255`                |
| Data directory        | The directory where data should be stored. This directory should not be used for anything else as it and all subdirectories will be automatically managed.                    | `--data-directory`        | `XFER_SERVER_DATA_DIRECTORY`    | `OS Data Directory/xfer-server` |
| Transfer expire after | Amount of time after-upload before a transfer is automatically deleted from storage. Upload expiry time will be sent to clients upon upload with the X-Xfer-ExpiresAt header. | `--transfer-expire-after` | `XFER_SERVER_TRANSFER_EXPIRE_AFTER`    | `1h`                            |
| Transfer size limit   | The maximum transfer size that is permitted.                                                                                                                                  | `--transfer-max-size`     | `XFER_SERVER_TRANSFER_MAX_SIZE` | `50MB`                          |
