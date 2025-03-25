# inv_sig_helper

`inv_sig_helper` is a Rust service that decrypts YouTube signatures and manages player information. It offers a TCP/Unix socket interface for signature decryption and related operations.

## Features

- Decrypt YouTube `n` and `s` signatures
- Fetch and update YouTube player data
- Provide signature timestamps and player status
- Efficient signature decryption with multi-threaded JavaScript execution

## Run with Docker (recommended method)

A Dockerfile is included for containerized deployment.

And an official Docker image is available at `quay.io/invidious/inv-sig-helper`: https://quay.io/repository/invidious/inv-sig-helper

### Production

Follow the official installation guide: https://github.com/iv-org/documentation/blob/master/docs/installation.md.

### Development

Run the project using docker compose:

```
docker compose up -d
```

Or you can run it manually but not recommended since you won't lock down the container from potential code execution from Google:

1. Build the image:

   ```
   docker build -t inv_sig_helper .
   ```

2. Run the container:

   ```
   docker run -p 127.0.0.1:12999:12999 inv_sig_helper
   ```

## Building and Running without Docker

### Prerequisites

- Rust 1.77 or later
- Cargo
- Patch
- openssl-devel

### Building

1. Clone the repository and navigate to the project directory:

   ```
   git clone https://github.com/iv-org/inv_sig_helper.git
   cd inv_sig_helper
   ```

2. Build the project:

   ```
   cargo build --release
   ```

### Running

#### Warning

This service runs untrusted code directly from Google.

We recommend running sig_helper inside a locked down environment like an LXC container or a systemd service where only the strict necessary is allowed. An examplary systemd service file is provided in `inv_sig_helper.service` which creates a socket in `/home/invidious/tmp/inv_sig_helper.sock`.

#### Instructions

The service can run in Unix socket mode (default) or TCP mode:

1. Unix socket mode:

   ```
   ./target/release/inv_sig_helper_rust
   ```

   This creates a Unix socket at `/tmp/inv_sig_helper.sock`.

2. TCP mode:

   ```
   ./target/release/inv_sig_helper_rust --tcp [IP:PORT]
   ```

   If no IP:PORT is given, it defaults to `127.0.0.1:12999`.

#### yt-dlp decoding

`inv_sig_helper` supports signature decoding using `yt-dlp`. This feature can be useful if you want to use `yt-dlp` for decoding signatures instead of the built-in code.

To enable decoding using `yt-dlp`, set the environment variable `USE_YT_DLP` to `1`:

```
export USE_YT_DLP=1
```

or uncomment the following line in your `docker-compose.yaml`:

```yaml
environment:
  - USE_YT_DLP=1  # use yt-dlp for decoding signatures instead of built-in code
```

inv_sig_helper.service:

```
Environment="USE_YT_DLP=1"
```

#### Troubleshooting

The log level can be configured using the `RUST_LOG` environment variable. Valid values are:

- error
- warn
- info
- debug
- trace

The `info` log level is the default setting. Changing this to `debug` will provide detailed logs on each request for additional troubleshooting.

In case of issues with decoding / `sig` function / `nsig` function, refer to the `yt-dlp decoding` section.


## Protocol Format

All data-types bigger than 1 byte are stored in network endian (big-endian) unless stated otherwise.

### Request Base
| Name      | Size (bytes) | Description                          |
|-----------|--------------|--------------------------------------|
|opcode     | 1            | The operation code to perform, A list of operations currently supported (and their data) can be found in the **Operations** chapter |
|request_id | 4            | The ID for the current request, Used to distinguish responses in the current connection |

The data afterwards depends on the supplied opcode, Please consult the **Operations** chapter for more information.

### Response Base
| Name       | Size (bytes) | Description                           |
|------------|--------------|---------------------------------------|
|request_id  | 4            | The ID for the request that this response is meant for |
|size        | 4            | Size of the response (excluding size of request id)|

The data afterwards depends on the supplied opcode, Please consult the **Operations** chapter for more information.

### Operations
#### `FORCE_UPDATE` (0x00)
Forces the server to re-fetch the YouTube player, and extract the necessary components from it (`nsig` function code, `sig` function code, signature timestamp).

##### Request
*No additional data required*

##### Response
| Name | Size (bytes) | Description |
|------|--------------|-------------|
|status| 2            | The status code of the request: `0xF44F` if successful, `0xFFFF` if no updating is required (YouTube's player ID is equal to the server's current player ID), `0x0000` if an error occurred |

#### `DECRYPT_N_SIGNATURE` (0x01)
Decrypt a provided `n` signature using the server's current `nsig` function code, and return the result (or an error).

##### Request
| Name | Size (bytes) | Description                         |
|------|--------------|-------------------------------------|
|size  | 2            | The size of the encrypted signature |
|string| *`size`*     | The encrypted signature             |

##### Response
| Name | Size (bytes) | Description                                                      |
|------|--------------|------------------------------------------------------------------|
|size  | 2            | The size of the decrypted signature, `0x0000` if an error occurred |
|string| *`size`*     | The decrypted signature                                          |

#### `DECRYPT_SIGNATURE` (0x02)
Decrypt a provided `s` signature using the server's current `sig` function code, and return the result (or an error).

##### Request
| Name | Size (bytes) | Description                         |
|------|--------------|-------------------------------------|
|size  | 2            | The size of the encrypted signature |
|string| *`size`*     | The encrypted signature             |

##### Response
| Name | Size (bytes) | Description                                                      |
|------|--------------|------------------------------------------------------------------|
|size  | 2            | The size of the decrypted signature, `0x0000` if an error occurred |
|string| *`size`*     | The decrypted signature                                          |

#### `GET_SIGNATURE_TIMESTAMP` (0x03)
Get the signature timestamp from the server's current player, and return it in the form of a 64-bit integer. If there's no player, it will return 0 in the `timestamp` (Please check with `PLAYER_STATUS` if the server has a player)

##### Request
No additional data required

##### Response
| Name    | Size (bytes) | Description                                              |
|---------|--------------|----------------------------------------------------------|
|timestamp| 8            | The signature timestamp from the server's current player |

#### `PLAYER_STATUS` (0x04)
Get the server's information about the current player.

##### Request
No additional data required

##### Response

| Name     | Size (bytes) | Description |
|----------|--------------|-------------|
|has_player| 1            | If the server has a player, this variable will be `0xFF`. or else, it will be `0x00`|
|player_id | 4            | The server's current player ID. If the server has no player, this will always be `0x00000000`|

#### `PLAYER_UPDATE_TIMESTAMP` (0x05)
Get the time of the last player update, The time is represented as seconds since the last update

##### Request
No additional data required

##### Response

| Name     | Size (bytes) | Description |
|----------|--------------|-------------|
|timestamp | 8            | Seconds since the last player update |

## License

This project is open source under the AGPL-3.0 license.
