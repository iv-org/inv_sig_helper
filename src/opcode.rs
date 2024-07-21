use std::io::ErrorKind;

use tokio_util::{
    bytes::{Buf, BufMut},
    codec::{Decoder, Encoder},
};

use crate::{jobs::JobOpcode, player::FetchUpdateStatus};

#[derive(Copy, Clone)]
pub struct OpcodeDecoder {}

pub struct Opcode {
    pub opcode: JobOpcode,
    pub request_id: u32,

    pub signature: String,
}

pub struct OpcodeResponse {
    pub opcode: JobOpcode,
    pub request_id: u32,

    pub update_status: Result<(), FetchUpdateStatus>,
    pub signature: String,
    pub signature_timestamp: u64,

    pub has_player: u8,
    pub player_id: u32,
    pub last_player_update: u64,
}

impl Default for OpcodeResponse {
    fn default() -> Self {
        OpcodeResponse {
            opcode: JobOpcode::ForceUpdate,
            request_id: 0,
            update_status: Ok(()),
            signature: String::new(),
            signature_timestamp: 0,
            has_player: 0,
            player_id: 0,
            last_player_update: 0,
        }
    }
}
impl Decoder for OpcodeDecoder {
    type Item = Opcode;
    type Error = std::io::Error;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        println!("Decoder length: {}", src.len());
        if 5 > src.len() {
            return Ok(None);
        }

        let opcode_byte: u8 = src[0];
        let opcode: JobOpcode = opcode_byte.into();
        let request_id: u32 = u32::from_be_bytes(src[1..5].try_into().unwrap());

        match opcode {
            JobOpcode::ForceUpdate
            | JobOpcode::GetSignatureTimestamp
            | JobOpcode::PlayerStatus
            | JobOpcode::PlayerUpdateTimestamp => {
                src.advance(5);
                Ok(Some(Opcode {
                    opcode,
                    request_id,
                    signature: Default::default(),
                }))
            }
            JobOpcode::DecryptSignature | JobOpcode::DecryptNSignature => {
                if 7 > src.len() {
                    return Ok(None);
                }

                let sig_size: u16 = ((src[5] as u16) << 8) | src[6] as u16;

                if (usize::from(sig_size) + 7) > src.len() {
                    return Ok(None);
                }

                let sig: String =
                    match String::from_utf8(src[7..(usize::from(sig_size) + 7)].to_vec()) {
                        Ok(x) => x,
                        Err(x) => {
                            return Err(std::io::Error::new(
                                ErrorKind::InvalidData,
                                x.utf8_error(),
                            ));
                        }
                    };

                src.advance(7 + sig.len());

                Ok(Some(Opcode {
                    opcode,
                    request_id,
                    signature: sig,
                }))
            }
            _ => Err(std::io::Error::new(ErrorKind::InvalidInput, "")),
        }
    }
}

impl Encoder<OpcodeResponse> for OpcodeDecoder {
    type Error = std::io::Error;
    fn encode(
        &mut self,
        item: OpcodeResponse,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        dst.put_u32(item.request_id);
        match item.opcode {
            JobOpcode::ForceUpdate => {
                dst.put_u32(2);
                match item.update_status {
                    Ok(_x) => dst.put_u16(0xF44F),
                    Err(FetchUpdateStatus::PlayerAlreadyUpdated) => dst.put_u16(0xFFFF),
                    Err(_x) => dst.put_u16(0x0000),
                }
            }
            JobOpcode::DecryptSignature | JobOpcode::DecryptNSignature => {
                dst.put_u32(2 + u32::try_from(item.signature.len()).unwrap());
                dst.put_u16(u16::try_from(item.signature.len()).unwrap());
                if !item.signature.is_empty() {
                    dst.put_slice(item.signature.as_bytes());
                }
            }
            JobOpcode::GetSignatureTimestamp => {
                dst.put_u32(8);
                dst.put_u64(item.signature_timestamp);
            }
            JobOpcode::PlayerStatus => {
                dst.put_u32(5);
                dst.put_u8(item.has_player);
                dst.put_u32(item.player_id);
            }
            JobOpcode::PlayerUpdateTimestamp => {
                dst.put_u32(8);
                dst.put_u64(item.last_player_update);
            }
            _ => {}
        }
        Ok(())
    }
}
