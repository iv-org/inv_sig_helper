# Protocol Format

All data-types bigger than 1 byte are stored in network endian (big-endian) unless stated otherwise.

## Request Base
| Name      | Size (bytes) | Description                          |
|-----------|--------------|--------------------------------------|
|opcode     | 1            | The operation code to perform, A list of operations currently supported (and their data) can be found in the **Operations** chapter |
|request_id | 4            | The ID for the current request, Used to distinguish responses in the current connection |

The data afterwards depends on the supplied opcode, Please consult the **Operations** chapter for more information.

## Response Base
| Name       | Size (bytes) | Description                           |
|------------|--------------|---------------------------------------|
|request_id  | 4            | The ID for the request that this response is meant for |
|size        | 4            | Size of the response (excluding size of request id)|

The data afterwards depends on the supplied opcode, Please consult the **Operations** chapter for more information.

## Operations
### `FORCE_UPDATE` (0x00)
Forces the server to re-fetch the YouTube player, and extract the necessary components from it (`nsig` function code, `sig` function code, signature timestamp).

#### Request
*No additional data required*

#### Response
| Name | Size (bytes) | Description |
|------|--------------|-------------|
|status| 2            | The status code of the request: `0xF44F` if successful, `0xFFFF` if no updating is required (YouTube's player ID is equal to the server's current player ID), `0x0000` if an error occurred |

### `DECRYPT_N_SIGNATURE` (0x01)
Decrypt a provided `n` signature using the server's current `nsig` function code, and return the result (or an error).

#### Request
| Name | Size (bytes) | Description                         |
|------|--------------|-------------------------------------|
|size  | 2            | The size of the encrypted signature |
|string| *`size`*     | The encrypted signature             |

#### Response
| Name | Size (bytes) | Description                                                      |
|------|--------------|------------------------------------------------------------------|
|size  | 2            | The size of the decrypted signature, `0x0000` if an error occurred |
|string| *`size`*     | The decrypted signature                                          |

### `DECRYPT_SIGNATURE` (0x02)
Decrypt a provided `s` signature using the server's current `sig` function code, and return the result (or an error).

#### Request
| Name | Size (bytes) | Description                         |
|------|--------------|-------------------------------------|
|size  | 2            | The size of the encrypted signature |
|string| *`size`*     | The encrypted signature             |

#### Response
| Name | Size (bytes) | Description                                                      |
|------|--------------|------------------------------------------------------------------|
|size  | 2            | The size of the decrypted signature, `0x0000` if an error occurred |
|string| *`size`*     | The decrypted signature                                          |

### `GET_SIGNATURE_TIMESTAMP` (0x03)
Get the signature timestamp from the server's current player, and return it in the form of a 64-bit integer. If there's no player, it will return 0 in the `timestamp` (Please check with `PLAYER_STATUS` if the server has a player)

#### Request
No additional data required

#### Response
| Name    | Size (bytes) | Description                                              |
|---------|--------------|----------------------------------------------------------|
|timestamp| 8            | The signature timestamp from the server's current player |

### `PLAYER_STATUS` (0x04)
Get the server's information about the current player.

#### Request
No additional data required

#### Response

| Name     | Size (bytes) | Description |
|----------|--------------|-------------|
|has_player| 1            | If the server has a player, this variable will be `0xFF`. or else, it will be `0x00`|
|player_id | 4            | The server's current player ID. If the server has no player, this will always be `0x00000000`|

