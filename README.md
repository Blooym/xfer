# xfer

> [!IMPORTANT]  
> **This project is makes no stability or security guarentees.**
>
> It is likely not to the optimal solution for your problem; Consider using [croc](https://github.com/schollz/croc/) instead.

Securely transfer files across the internet with end-to-end encryption.

*You are viewing the documentation for the xfer client, xfer server documentation can be found [here](./xfer-server/README.md).*

## Features

- End-to-end encrypted and sent via relay server.
- Supports uploading files and folders with full metadata retention.
- Efficient compression for faster transfers.

## Installation

*More installation options may become available in the future.*

### Using Cargo

```sh
cargo install --git https://github.com/Blooym/xfer.git xfer
```

## Usage

These examples will assume there is an xfer server at `https://example.com`. Please replace this with the url of the xfer server you actually want to use. For more in-depth information about flags and usage refer to the `xfer help` command.

### Transfer a file

```sh
$ xfer upload ./essay.txt -s https://example.com
```

### Transfer a folder

```sh
$ xfer upload ./photos -s https://example.com
```

Note that when creating a directory transfer all subdirectories will also be included.

### Download a transfer

```sh
$ xfer download <transfer_id> -s https://example.com -o ./xfer-downloads
```

When downloading a transfer files will be placed in the output directory, and folders will have their root folder placed in the output directory.

## Xfer Server Directory

***Note:*** *Although xfer encrypts data client-side, you should still have some trust in the server you use to faciliate your transfer.*

There are currently no public xfer servers. You can run your own by reading the [xfer server documentation](./xfer-server//README.md).
