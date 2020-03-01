
use std::io::{Error, ErrorKind};
use byteorder::{ByteOrder, LittleEndian, BigEndian, WriteBytesExt};


/// A packet is just a wrapper for a Vec<u8>
/// For reference, see https://dev.mysql.com/doc/internals/en/mysql-packet.html
#[derive(Debug,Clone,PartialEq)]
pub struct Packet {
  db_type: DatabaseType,
  pub bytes: Vec<u8>,
}

impl Packet {

  pub fn new(db_type: DatabaseType, bytes: Vec<u8>) -> Packet {
    Packet {
      db_type: db_type,
      bytes: bytes,
    }
  }

  /**
  * Create an error packet for MariaDB
  **/
  pub fn error_packet_mariadb(code: u16, state: [u8; 5], msg: String) -> Self {

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
    Packet { db_type: DatabaseType::MariaDB, bytes: header }
  }

  pub fn get_query(&self) -> Result<String, Error>{
    match (self.db_type, self.get_packet_type()) {
      (DatabaseType::MariaDB, Ok(PacketType::ComQuery)) => Ok(String::from_utf8(self.bytes[5..].to_vec()).expect("Invalid UTF-8")),
      (DatabaseType::PostgresSQL, Ok(PacketType::Query)) => Ok(String::from_utf8(self.bytes[5..].to_vec()).expect("Invalid UTF-8")),
      _ => Err(Error::new(ErrorKind::Other, "Packet is not a query")),
    }
  }

  pub fn get_sequence_id(&self) -> Result<u8, Error> {
    match self.db_type {
      DatabaseType::MariaDB => Ok(self.bytes[3]),
      DatabaseType::PostgresSQL => Err(Error::new(ErrorKind::Other, "PostgresSQL does not use sequence IDs")),
    }
  }

  /// Determine the type of packet
  pub fn get_packet_type(&self) -> Result<PacketType, Error> {
    match self.db_type {
      // https://dev.mysql.com/doc/internals/en/mysql-packet.html
      // https://dev.mysql.com/doc/internals/en/text-protocol.html
      DatabaseType::MariaDB => match self.bytes[4] {
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
      },

      // https://www.postgresql.org/docs/12/protocol-message-types.html
      // https://www.postgresql.org/docs/12/protocol-message-formats.html
      DatabaseType::PostgresSQL => match self.bytes[0] as char {
        'R' => {
          if self.bytes.len() < 9 {
            return Err(Error::new(ErrorKind::Other, "Invalid packet type: Authentication Packet too short"))
          }
          let length = BigEndian::read_u32(&self.bytes[1..5]);
          let payload = BigEndian::read_u32(&self.bytes[5..9]);
          match (length, payload) {
            (8, 0) => Ok(PacketType::AuthenticationOk),
            (8, 2) => Ok(PacketType::AuthenticationKerberosV5),
            (8, 3) => Ok(PacketType::AuthenticationCleartextPassword),
            (12, 5) => Ok(PacketType::AuthenticationMD5Password),
            (8, 6) => Ok(PacketType::AuthenticationSCMCredential),
            (8, 7) => Ok(PacketType::AuthenticationGSS),
            (8, 9) => Ok(PacketType::AuthenticationSSPI),
            (_, 8) => Ok(PacketType::AuthenticationGSSContinue),
            (_, 10) => Ok(PacketType::AuthenticationSASL),
            (_, 11) => Ok(PacketType::AuthenticationSASLContinue),
            (_, 12) => Ok(PacketType::AuthenticationSASLFinal),
            _ => Err(Error::new(ErrorKind::Other, "Invalid packet type: Authentication Packet unrecognized")),
          }
        },
        'K' => Ok(PacketType::BackendKeyData),
        'B' => Ok(PacketType::Bind),
        '2' => Ok(PacketType::BindComplete),
        '3' => Ok(PacketType::CloseComplete),
        'C' => {
          if self.bytes.len() < 6 {
            return Err(Error::new(ErrorKind::Other, "Invalid packet type: Close/CommandComplete packet too short"))
          } else if self.bytes[5] as char == 'S' || self.bytes[5] as char == 'P' {
            return Ok(PacketType::Close)
          } else {
            return Ok(PacketType::CommandComplete)
          }
        },
        'd' => Ok(PacketType::CopyData),
        'c' => Ok(PacketType::CopyDone),
        'f' => Ok(PacketType::CopyFail),
        'G' => Ok(PacketType::CopyInResponse),
        'H' => {
          if self.bytes.len() < 5 {
            return Err(Error::new(ErrorKind::Other, "Invalid packet type: Authentication Packet too short"))
          }
          let length = BigEndian::read_u32(&self.bytes[1..5]);
          if length == 4 {
            return Ok(PacketType::Flush)
          } else {
            return Ok(PacketType::CopyOutResponse)
          }
        },
        'W' => Ok(PacketType::CopyBothResponse),
        'D' => {
          if self.bytes.len() < 6 {
            return Err(Error::new(ErrorKind::Other, "Invalid packet type: DataRow/Describe packet too short"))
          } else if self.bytes[5] as char == 'S' || self.bytes[5] as char == 'P' {
            return Ok(PacketType::Describe)
          } else {
            return Ok(PacketType::DataRow)
          }
        },
        'I' => Ok(PacketType::EmptyQueryResponse),
        'E' => {
          if self.bytes.len() < 6 {
            return Err(Error::new(ErrorKind::Other, "Invalid packet type: Execute/ErrorResponse packet too short"))
          // https://www.postgresql.org/docs/12/protocol-error-fields.html
          } else if self.bytes[5] as char == 'S' || 
            self.bytes[5] as char == 'V' ||
            self.bytes[5] as char == 'V' ||
            self.bytes[5] as char == 'C' ||
            self.bytes[5] as char == 'M' ||
            self.bytes[5] as char == 'D' ||
            self.bytes[5] as char == 'H' ||
            self.bytes[5] as char == 'P' ||
            self.bytes[5] as char == 'p' ||
            self.bytes[5] as char == 'q' ||
            self.bytes[5] as char == 'W' ||
            self.bytes[5] as char == 's' ||
            self.bytes[5] as char == 't' ||
            self.bytes[5] as char == 'c' ||
            self.bytes[5] as char == 'd' ||
            self.bytes[5] as char == 'n' ||
            self.bytes[5] as char == 'F' ||
            self.bytes[5] as char == 'L' ||
            self.bytes[5] as char == 'R' ||
            self.bytes[5] as char == 'P' {
            return Ok(PacketType::ErrorResponse)
          } else {
            return Ok(PacketType::Execute)
          }

        },
        'F' => Ok(PacketType::FunctionCall),
        'V' => Ok(PacketType::FunctionCallResponse),
        'p' => Ok(PacketType::AuthenticationResponse),
        'v' => Ok(PacketType::NegotiateProtocolVersion),
        'n' => Ok(PacketType::NoData),
        'N' => Ok(PacketType::NoticeResponse),
        'A' => Ok(PacketType::NotificationResponse),
        't' => Ok(PacketType::ParameterDescription),
        'S' => {
          if self.bytes.len() < 5 {
            return Err(Error::new(ErrorKind::Other, "Invalid packet type: Sync/ParameterStatus Packet too short"));
          }
          let length = BigEndian::read_u32(&self.bytes[1..5]);
          if length == 4 {
            return Ok(PacketType::Sync)
          } else {
            return Ok(PacketType::ParameterStatus)
          }

        },
        'P' => Ok(PacketType::Parse),
        '1' => Ok(PacketType::ParseComplete),
        's' => Ok(PacketType::PortalSuspended),
        'Q' => Ok(PacketType::Query),
        'Z' => Ok(PacketType::ReadyForQuery),
        'T' => Ok(PacketType::RowDescription),
        'X' => Ok(PacketType::Terminate),
        _ => {
          if self.bytes.len() < 8 {
            return Err(Error::new(ErrorKind::Other, "Invalid packet type: Default packet too short"));
          }
          let length = BigEndian::read_u32(&self.bytes[0..4]);
          let payload = BigEndian::read_u32(&self.bytes[4..8]);
          match (length, payload) {
            (16, 80877102) => Ok(PacketType::CancelRequest),
            (8, 80877103) => Ok(PacketType::SSLRequest),
            (8, 80877104) => Ok(PacketType::GSSENCRequest),
            (_, 196608) => Ok(PacketType::StartupMessage),
            _ => Err(Error::new(ErrorKind::Other, "Invalid packet type")),
          }
        }
      } // end match packet_type
    } // end match db_type
  } // end fn

}

#[derive(Copy,Clone,Debug,PartialEq)]
pub enum DatabaseType {
  MariaDB,
  PostgresSQL,
}

#[derive(Copy,Clone,Debug)]
pub enum PacketType {
  // MariaDB
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

  //PostgresSQL
  AuthenticationOk,
  AuthenticationKerberosV5,
  AuthenticationCleartextPassword,
  AuthenticationMD5Password,
  AuthenticationSCMCredential,
  AuthenticationGSS,
  AuthenticationSSPI,
  AuthenticationGSSContinue,
  AuthenticationSASL,
  AuthenticationSASLContinue,
  AuthenticationSASLFinal,
  AuthenticationResponse, //GSSResponse,PasswordMessage,SASLInitialResponse,SASLResponse
  BackendKeyData,
  Bind,
  BindComplete,
  CancelRequest,
  Close,
  CloseComplete,
  CommandComplete,
  CopyData,
  CopyDone,
  CopyFail,
  CopyInResponse,
  CopyOutResponse,
  CopyBothResponse,
  DataRow,
  Describe,
  EmptyQueryResponse,
  ErrorResponse,
  Execute,
  Flush,
  FunctionCall,
  FunctionCallResponse,
  GSSResponse,
  NegotiateProtocolVersion,
  NoData,
  NoticeResponse,
  NotificationResponse,
  ParameterDescription,
  ParameterStatus,
  Parse,
  ParseComplete,
  PasswordMessage,
  PortalSuspended,
  Query,
  ReadyForQuery,
  RowDescription,
  SASLInitialResponse,
  SASLResponse,
  SSLRequest,
  GSSENCRequest,
  StartupMessage,
  Sync,
  Terminate,
}



