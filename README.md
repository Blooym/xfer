# xfer

> [!IMPORTANT]  
> **This project makes no stability or security guarentees.**
>
> It is likely not to the optimal solution for your problem; Consider using [croc](https://github.com/schollz/croc/) instead.

Transfer files and folders between devices via the internet with end-to-end encryption.

*You are viewing the documentation for the xfer client, xfer server documentation can be found [here](./xfer-server/README.md).*

## Features

- Transfers are end-to-end encrypted and sent via relay server.
- Upload files or folders with full metadata retention.
- Efficient compression of content for faster transfers.

## Installation

*More installation options may become available in the future.*

### Using Cargo

```sh
cargo install --git https://github.com/Blooym/xfer.git xfer
```

## Usage

These examples will assume you're using the default xfer server. Use the `--server <URL>` flag when uploading or downloading to use a custom server.

*For more in-depth information about commands and flags, refer to the `xfer help` command.*

### Transfer a file

```sh
$ xfer upload ./essay.txt
```

### Transfer a folder

```sh
$ xfer upload ./photos
```

Note that when creating a directory transfer all subdirectories will also be included.

### Download a transfer

```sh
$ xfer download <transfer_id> -o ./xfer-downloads
```

When downloading a transfer files will be placed in the output directory, and folders will have their root folder placed in the output directory.

## Xfer Server Directory

*Available servers will change with time; If the default server is shut down an update will be pushed to remove it as the default.*

| Server            | Notes                                                                                                                  | Country | Operator                           |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------- | ------- | ------------------------------------ |
| `xfer.blooym.dev` | Very limited max upload size, low transfer retention time and ratelimits on how many transfers can be made in a time period. | Germany | [@Blooym](https://github.com/Blooym) |


Want to host your own? Learn more about running an xfer server by reading the [xfer server documentation](./xfer-server//README.md).
