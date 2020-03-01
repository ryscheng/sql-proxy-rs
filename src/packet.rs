use std::{
    convert::TryFrom as _,
    io::{Error, ErrorKind},
};

use byteorder::{LittleEndian, WriteBytesExt};

/// A packet is just a wrapper for a Vec<u8>
/// For reference, see https://dev.mysql.com/doc/internals/en/mysql-packet.html
#[derive(Debug, PartialEq)]
pub struct Packet {
    pub bytes: Vec<u8>,
}

impl Packet {
    /**
     * Create an error packet
     **/
    pub fn error_packet(code: u16, state: [u8; 5], msg: String) -> Self {
        // start building payload
        let mut payload: Vec<u8> = Vec::with_capacity(9 + msg.len());
        payload.push(0xff); // packet type
        payload.write_u16::<LittleEndian>(code).unwrap(); // error code
        payload.extend_from_slice(b"#"); // sql_state_marker
        payload.extend_from_slice(&state); // SQL STATE
        payload.extend_from_slice(msg.as_bytes());

        // create header with length and sequence id
        let mut header: Vec<u8> = Vec::with_capacity(4 + 9 + msg.len());
        header
            .write_u32::<LittleEndian>(payload.len() as u32)
            .unwrap();
        header.pop(); // we need 3 byte length, so discard last byte
        header.push(1); // sequence_id

        // combine the vectors
        header.extend_from_slice(&payload);

        // now move the vector into the packet
        Packet { bytes: header }
    }

    pub fn sequence_id(&self) -> u8 {
        self.bytes[3]
    }

    /// Determine the type of packet
    pub fn packet_type(&self) -> Result<PacketType, Error> {
        PacketType::try_from(self.bytes[4])
            .map_err(|_| Error::new(ErrorKind::Other, "Invalid packet type"))
    }
}

#[derive(Copy, Clone, Debug, proper::Prim)]
#[prim(ty = "u8")]
pub enum PacketType {
    ComSleep,
    ComQuit,
    ComInitDb,
    ComQuery,
    ComFieldList,
    ComCreateDb,
    ComDropDb,
    ComRefresh,
    ComShutdown,
    ComStatistics,
    ComProcessInfo,
    ComConnect,
    ComProcessKill,
    ComDebug,
    ComPing,
    ComTime,
    ComDelayedInsert,
    ComChangeUser,
    ComBinlogDump,
    ComTableDump,
    ComConnectOut,
    ComRegisterSlave,
    ComStmtPrepare,
    ComStmtExecute,
    ComStmtSendLongData,
    ComStmtClose,
    ComStmtReset,
    ComDaemon,
    ComBinlogDumpGtid,
    ComResetConnection,
}
