use arrayvec::CapacityError;
use core::fmt;
use core::str::Utf8Error;

use serde;

/// The result of a serialization or deserialization operation.
pub type Result<T> = ::core::result::Result<T, Error>;

/// An error that can be produced during (de)serializing.
pub type Error = ErrorKind;

/// The kind of error that can be produced during a serialization or deserialization.
#[derive(Debug)]
pub enum ErrorKind {
    // /// If the error stems from the reader/writer that is being used
    // /// during (de)serialization, that error will be stored and returned here.
    // Io(io::Error),
    Fmt(fmt::Error),
    /// Returned if the deserializer attempts to deserialize a string that is not valid utf8
    InvalidUtf8Encoding(Utf8Error),
    /// Returned if the deserializer attempts to deserialize a bool that was
    /// not encoded as either a 1 or a 0
    InvalidBoolEncoding(u8),
    /// Returned if the deserializer attempts to deserialize a char that is not in the correct format.
    InvalidCharEncoding,
    /// Returned if the deserializer attempts to deserialize the tag of an enum that is
    /// not in the expected ranges
    InvalidTagEncoding(usize),
    /// Serde has a deserialize_any method that lets the format hint to the
    /// object which route to take in deserializing.
    DeserializeAnyNotSupported,
    /// If (de)serializing a message takes more than the provided size limit, this
    /// error is returned.
    SizeLimit,
    /// Bincode can not encode sequences of unknown length (like iterators).
    SequenceMustHaveLength,
    // /// A custom error message from Serde.
    // Custom(String),
    CapacityError(CapacityError<u8>),
    Serde,
}

// impl StdError for ErrorKind {
//     fn description(&self) -> &str {
//         match *self {
//             ErrorKind::Io(ref err) => error::Error::description(err),
//             ErrorKind::InvalidUtf8Encoding(_) => "string is not valid utf8",
//             ErrorKind::InvalidBoolEncoding(_) => "invalid u8 while decoding bool",
//             ErrorKind::InvalidCharEncoding => "char is not valid",
//             ErrorKind::InvalidTagEncoding(_) => "tag for enum is not valid",
//             ErrorKind::SequenceMustHaveLength => {
//                 "Bincode can only encode sequences and maps that have a knowable size ahead of time"
//             }
//             ErrorKind::DeserializeAnyNotSupported => {
//                 "Bincode doesn't support serde::Deserializer::deserialize_any"
//             }
//             ErrorKind::SizeLimit => "the size limit has been reached",
//             ErrorKind::Custom(ref msg) => msg,
//         }
//     }

//     fn cause(&self) -> Option<&error::Error> {
//         match *self {
//             ErrorKind::Io(ref err) => Some(err),
//             ErrorKind::InvalidUtf8Encoding(_) => None,
//             ErrorKind::InvalidBoolEncoding(_) => None,
//             ErrorKind::InvalidCharEncoding => None,
//             ErrorKind::InvalidTagEncoding(_) => None,
//             ErrorKind::SequenceMustHaveLength => None,
//             ErrorKind::DeserializeAnyNotSupported => None,
//             ErrorKind::SizeLimit => None,
//             ErrorKind::Custom(_) => None,
//         }
//     }
// }

// impl From<io::Error> for Error {
//     fn from(err: io::Error) -> Error {
//         ErrorKind::Io(err).into()
//     }
// }

impl From<fmt::Error> for Error {
    fn from(err: fmt::Error) -> Error {
        ErrorKind::Fmt(err).into()
    }
}

impl From<CapacityError<u8>> for Error {
    fn from(err: CapacityError<u8>) -> Error {
        ErrorKind::CapacityError(err)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // ErrorKind::Io(ref ioerr) => write!(fmt, "io error: {}", ioerr),
            ErrorKind::InvalidUtf8Encoding(e) => write!(fmt, "{}: {}", self, e),
            ErrorKind::InvalidBoolEncoding(b) => {
                write!(fmt, "{}, expected 0 or 1, found {}", self, b)
            }
            ErrorKind::InvalidCharEncoding => write!(fmt, "{}", self),
            ErrorKind::InvalidTagEncoding(tag) => write!(fmt, "{}, found {}", self, tag),
            ErrorKind::SequenceMustHaveLength => write!(fmt, "{}", self),
            ErrorKind::SizeLimit => write!(fmt, "{}", self),
            ErrorKind::DeserializeAnyNotSupported => write!(
                fmt,
                "Bincode does not support the serde::Deserializer::deserialize_any method"
            ),
            ErrorKind::CapacityError(c) => write!(fmt, "{}", c),
            ErrorKind::Fmt(f) => write!(fmt, "{}", f),
            ErrorKind::Serde => write!(fmt, "Serde error"),
        }
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        ErrorKind::Serde
    }
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(_msg: T) -> Self {
        ErrorKind::Serde
    }
}
