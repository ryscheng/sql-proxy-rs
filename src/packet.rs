
use std::io::{self, Read, Write, Error, ErrorKind};
use byteorder::{LittleEndian, WriteBytesExt};

/// A packet is just a wrapper for a Vec<u8>
#[derive(Debug,PartialEq)]
pub struct Packet {
    pub bytes: Vec<u8>
}

impl Packet {

    /**
     * Create an error packet
     **/
    pub fn error_packet(code: u16, state: [u8; 5], msg: String) -> Self {

        // start building payload
        let mut payload: Vec<u8> = Vec::with_capacity(9 + msg.len());
        payload.push(0xff);  // packet type
        payload.write_u16::<LittleEndian>(code).unwrap(); // error code
        payload.extend_from_slice("#".as_bytes()); // sql_state_marker
        payload.extend_from_slice(&state); // SQL STATE
        payload.extend_from_slice(msg.as_bytes());

        // create header with length and sequence id
        let mut header: Vec<u8> = Vec::with_capacity(4 + 9 + msg.len());
        header.write_u32::<LittleEndian>(payload.len() as u32).unwrap();
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
        match self.bytes[4] {
            0x00 => Ok(PacketType::ComSleep),
            0x01 => Ok(PacketType::ComQuit),
            0x02 => Ok(PacketType::ComInitDb),
            0x03 => Ok(PacketType::ComQuery),
            0x04 => Ok(PacketType::ComFieldList),
            0x05 => Ok(PacketType::ComCreateDb),
            0x06 => Ok(PacketType::ComDropDb),
            0x07 => Ok(PacketType::ComRefresh),
            0x08 => Ok(PacketType::ComShutdown),
            0x09 => Ok(PacketType::ComStatistics),
            0x0a => Ok(PacketType::ComProcessInfo),
            0x0b => Ok(PacketType::ComConnect),
            0x0c => Ok(PacketType::ComProcessKill),
            0x0d => Ok(PacketType::ComDebug),
            0x0e => Ok(PacketType::ComPing),
            0x0f => Ok(PacketType::ComTime),
            0x10 => Ok(PacketType::ComDelayedInsert),
            0x11 => Ok(PacketType::ComChangeUser),
            0x12 => Ok(PacketType::ComBinlogDump),
            0x13 => Ok(PacketType::ComTableDump),
            0x14 => Ok(PacketType::ComConnectOut),
            0x15 => Ok(PacketType::ComRegisterSlave),
            0x16 => Ok(PacketType::ComStmtPrepare),
            0x17 => Ok(PacketType::ComStmtExecute),
            0x18 => Ok(PacketType::ComStmtSendLongData),
            0x19 => Ok(PacketType::ComStmtClose),
            0x1a => Ok(PacketType::ComStmtReset),
            0x1d => Ok(PacketType::ComDaemon),
            0x1e => Ok(PacketType::ComBinlogDumpGtid),
            0x1f => Ok(PacketType::ComResetConnection),
            _ => Err(Error::new(ErrorKind::Other, "Invalid packet type"))
        }
    }

}

#[derive(Copy,Clone)]
pub enum PacketType {
    ComSleep = 0x00,
    ComQuit = 0x01,
    ComInitDb = 0x02,
    ComQuery = 0x03,
    ComFieldList = 0x04,
    ComCreateDb = 0x05,
    ComDropDb = 0x06,
    ComRefresh = 0x07,
    ComShutdown = 0x08,
    ComStatistics = 0x09,
    ComProcessInfo = 0x0a,
    ComConnect = 0x0b,
    ComProcessKill= 0x0c,
    ComDebug = 0x0d,
    ComPing = 0x0e,
    ComTime = 0x0f,
    ComDelayedInsert = 0x10,
    ComChangeUser = 0x11,
    ComBinlogDump = 0x12,
    ComTableDump = 0x13,
    ComConnectOut = 0x14,
    ComRegisterSlave = 0x15,
    ComStmtPrepare = 0x16,
    ComStmtExecute = 0x17,
    ComStmtSendLongData = 0x18,
    ComStmtClose = 0x19,
    ComStmtReset = 0x1a,
    ComDaemon= 0x1d,
    ComBinlogDumpGtid = 0x1e,
    ComResetConnection = 0x1f,
}

