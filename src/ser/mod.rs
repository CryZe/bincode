use core::u32;

use arrayvec::{Array, ArrayVec};
use serde;

// use byteorder::WriteBytesExt;
use byteorder::ByteOrder;

use super::internal::SizeLimit;
use super::{Error, ErrorKind, Result};
use config::Options;
use core::fmt::{Display, Write};

/// An Serializer that encodes values directly into a Writer.
///
/// The specified byte-order will impact the endianness that is
/// used during the encoding.
///
/// This struct should not be used often.
/// For most cases, prefer the `encode_into` function.
pub(crate) struct Serializer<'w, A: Array<Item = u8> + 'w, O: Options> {
    writer: &'w mut ArrayVec<A>,
    _options: O,
}

impl<'w, A: Array<Item = u8>, O: Options> Serializer<'w, A, O> {
    /// Creates a new Serializer with the given `Write`r.
    pub fn new(w: &'w mut ArrayVec<A>, options: O) -> Serializer<'w, A, O> {
        Serializer {
            writer: w,
            _options: options,
        }
    }
}

impl<'a, 'w, A: Array<Item = u8>, O: Options> serde::Serializer for &'a mut Serializer<'w, A, O> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Compound<'a, 'w, A, O>;
    type SerializeTuple = Compound<'a, 'w, A, O>;
    type SerializeTupleStruct = Compound<'a, 'w, A, O>;
    type SerializeTupleVariant = Compound<'a, 'w, A, O>;
    type SerializeMap = Compound<'a, 'w, A, O>;
    type SerializeStruct = Compound<'a, 'w, A, O>;
    type SerializeStructVariant = Compound<'a, 'w, A, O>;

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.writer
            .try_push(if v { 1 } else { 0 })
            .map_err(Into::into)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.writer.try_push(v).map_err(Into::into)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        let mut buf = [0; 2];
        O::Endian::write_u16(&mut buf, v);
        for &val in &buf {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        let mut buf = [0; 4];
        O::Endian::write_u32(&mut buf, v);
        for &val in &buf {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        let mut buf = [0; 8];
        O::Endian::write_u64(&mut buf, v);
        for &val in &buf {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.writer.try_push(v as u8).map_err(Into::into)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        let mut buf = [0; 2];
        O::Endian::write_i16(&mut buf, v);
        for &val in &buf {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        let mut buf = [0; 4];
        O::Endian::write_i32(&mut buf, v);
        for &val in &buf {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        let mut buf = [0; 8];
        O::Endian::write_i64(&mut buf, v);
        for &val in &buf {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    #[cfg(feature = "i128")]
    fn serialize_u128(self, v: u128) -> Result<()> {
        self.writer.write_u128::<O::Endian>(v).map_err(Into::into)
    }

    #[cfg(feature = "i128")]
    fn serialize_i128(self, v: i128) -> Result<()> {
        self.writer.write_i128::<O::Endian>(v).map_err(Into::into)
    }

    serde_if_integer128! {
        #[cfg(not(feature = "i128"))]
        fn serialize_u128(self, v: u128) -> Result<()> {
            let _ = v;
            panic!("u128 is not supported. Enable the `i128` feature of `bincode`")
        }

        #[cfg(not(feature = "i128"))]
        fn serialize_i128(self, v: i128) -> Result<()> {
            let _ = v;
            panic!("i128 is not supported. Enable the `i128` feature of `bincode`")
        }
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        let mut buf = [0; 4];
        O::Endian::write_f32(&mut buf, v);
        for &val in &buf {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        let mut buf = [0; 8];
        O::Endian::write_f64(&mut buf, v);
        for &val in &buf {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn collect_str<T: ?Sized>(self, value: &T) -> Result<()>
    where
        T: Display,
    {
        let pos = self.writer.len();
        try!(self.serialize_u64(0));
        write!(ArrayVecWrite(self.writer), "{}", value)?;
        let new_pos = self.writer.len();
        let len = new_pos - pos - 8;
        O::Endian::write_u64(&mut self.writer[pos..], len as u64);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        try!(self.serialize_u64(v.len() as u64));
        for &val in v.as_bytes() {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn serialize_char(self, c: char) -> Result<()> {
        for &val in encode_utf8(c).as_slice() {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        try!(self.serialize_u64(v.len() as u64));
        for &val in v {
            self.writer.try_push(val)?;
        }
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        self.writer.try_push(0)?;
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, v: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.writer.try_push(1)?;
        v.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let len = try!(len.ok_or(ErrorKind::SequenceMustHaveLength));
        try!(self.serialize_u64(len as u64));
        Ok(Compound { ser: self })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(Compound { ser: self })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(Compound { ser: self })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        try!(self.serialize_u32(variant_index));
        Ok(Compound { ser: self })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        let len = try!(len.ok_or(ErrorKind::SequenceMustHaveLength));
        try!(self.serialize_u64(len as u64));
        Ok(Compound { ser: self })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(Compound { ser: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        try!(self.serialize_u32(variant_index));
        Ok(Compound { ser: self })
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        try!(self.serialize_u32(variant_index));
        value.serialize(self)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.serialize_u32(variant_index)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub(crate) struct SizeChecker<O: Options> {
    pub options: O,
}

impl<O: Options> SizeChecker<O> {
    pub fn new(options: O) -> SizeChecker<O> {
        SizeChecker { options: options }
    }

    fn add_raw(&mut self, size: u64) -> Result<()> {
        self.options.limit().add(size)
    }

    fn add_value<T>(&mut self, t: T) -> Result<()> {
        use core::mem::size_of_val;
        self.add_raw(size_of_val(&t) as u64)
    }
}

use core::fmt;

struct ArrayVecWrite<'a, A: Array<Item = u8> + 'a>(&'a mut ArrayVec<A>);

impl<'a, A: Array<Item = u8>> fmt::Write for ArrayVecWrite<'a, A> {
    fn write_str(&mut self, s: &str) -> ::core::result::Result<(), fmt::Error> {
        for &b in s.as_bytes() {
            self.0.try_push(b).map_err(|_| fmt::Error)?;
        }
        Ok(())
    }
}

struct CountWrite(usize);

impl fmt::Write for CountWrite {
    fn write_str(&mut self, s: &str) -> ::core::result::Result<(), fmt::Error> {
        self.0 += s.len();
        Ok(())
    }
}

impl<'a, O: Options> serde::Serializer for &'a mut SizeChecker<O> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SizeCompound<'a, O>;
    type SerializeTuple = SizeCompound<'a, O>;
    type SerializeTupleStruct = SizeCompound<'a, O>;
    type SerializeTupleVariant = SizeCompound<'a, O>;
    type SerializeMap = SizeCompound<'a, O>;
    type SerializeStruct = SizeCompound<'a, O>;
    type SerializeStructVariant = SizeCompound<'a, O>;

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_bool(self, _: bool) -> Result<()> {
        self.add_value(0 as u8)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.add_value(v)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.add_value(v)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.add_value(v)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.add_value(v)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.add_value(v)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.add_value(v)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.add_value(v)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.add_value(v)
    }

    serde_if_integer128! {
        fn serialize_u128(self, v: u128) -> Result<()> {
            self.add_value(v)
        }

        fn serialize_i128(self, v: i128) -> Result<()> {
            self.add_value(v)
        }
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.add_value(v)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.add_value(v)
    }

    fn collect_str<T: ?Sized>(self, value: &T) -> Result<()>
    where
        T: Display,
    {
        self.add_value(0 as u64)?;
        let mut count_write = CountWrite(0);
        write!(&mut count_write, "{}", value)?;
        self.add_raw(count_write.0 as u64);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        try!(self.add_value(0 as u64));
        self.add_raw(v.len() as u64)
    }

    fn serialize_char(self, c: char) -> Result<()> {
        self.add_raw(encode_utf8(c).as_slice().len() as u64)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        try!(self.add_value(0 as u64));
        self.add_raw(v.len() as u64)
    }

    fn serialize_none(self) -> Result<()> {
        self.add_value(0 as u8)
    }

    fn serialize_some<T: ?Sized>(self, v: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        try!(self.add_value(1 as u8));
        v.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let len = try!(len.ok_or(ErrorKind::SequenceMustHaveLength));

        try!(self.serialize_u64(len as u64));
        Ok(SizeCompound { ser: self })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(SizeCompound { ser: self })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SizeCompound { ser: self })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        try!(self.add_value(variant_index));
        Ok(SizeCompound { ser: self })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        let len = try!(len.ok_or(ErrorKind::SequenceMustHaveLength));

        try!(self.serialize_u64(len as u64));
        Ok(SizeCompound { ser: self })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(SizeCompound { ser: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        try!(self.add_value(variant_index));
        Ok(SizeCompound { ser: self })
    }

    fn serialize_newtype_struct<V: serde::Serialize + ?Sized>(
        self,
        _name: &'static str,
        v: &V,
    ) -> Result<()> {
        v.serialize(self)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.add_value(variant_index)
    }

    fn serialize_newtype_variant<V: serde::Serialize + ?Sized>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &V,
    ) -> Result<()> {
        try!(self.add_value(variant_index));
        value.serialize(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

pub(crate) struct Compound<'a, 'w: 'a, A: Array<Item = u8> + 'w + 'a, O: Options + 'a> {
    ser: &'a mut Serializer<'w, A, O>,
}

impl<'a, 'w, A, O> serde::ser::SerializeSeq for Compound<'a, 'w, A, O>
where
    A: Array<Item = u8>,
    O: Options,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, A, O> serde::ser::SerializeTuple for Compound<'a, 'w, A, O>
where
    A: Array<Item = u8>,
    O: Options,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, A, O> serde::ser::SerializeTupleStruct for Compound<'a, 'w, A, O>
where
    A: Array<Item = u8>,
    O: Options,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, A, O> serde::ser::SerializeTupleVariant for Compound<'a, 'w, A, O>
where
    A: Array<Item = u8>,
    O: Options,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, A, O> serde::ser::SerializeMap for Compound<'a, 'w, A, O>
where
    A: Array<Item = u8>,
    O: Options,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<K: ?Sized>(&mut self, value: &K) -> Result<()>
    where
        K: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn serialize_value<V: ?Sized>(&mut self, value: &V) -> Result<()>
    where
        V: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, A, O> serde::ser::SerializeStruct for Compound<'a, 'w, A, O>
where
    A: Array<Item = u8>,
    O: Options,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, 'w, A, O> serde::ser::SerializeStructVariant for Compound<'a, 'w, A, O>
where
    A: Array<Item = u8>,
    O: Options,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

pub(crate) struct SizeCompound<'a, S: Options + 'a> {
    ser: &'a mut SizeChecker<S>,
}

impl<'a, O: Options> serde::ser::SerializeSeq for SizeCompound<'a, O> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, O: Options> serde::ser::SerializeTuple for SizeCompound<'a, O> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, O: Options> serde::ser::SerializeTupleStruct for SizeCompound<'a, O> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, O: Options> serde::ser::SerializeTupleVariant for SizeCompound<'a, O> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, O: Options + 'a> serde::ser::SerializeMap for SizeCompound<'a, O> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<K: ?Sized>(&mut self, value: &K) -> Result<()>
    where
        K: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn serialize_value<V: ?Sized>(&mut self, value: &V) -> Result<()>
    where
        V: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, O: Options> serde::ser::SerializeStruct for SizeCompound<'a, O> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, O: Options> serde::ser::SerializeStructVariant for SizeCompound<'a, O> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T: ?Sized>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: serde::ser::Serialize,
    {
        value.serialize(&mut *self.ser)
    }

    #[inline]
    fn end(self) -> Result<()> {
        Ok(())
    }
}
const TAG_CONT: u8 = 0b1000_0000;
const TAG_TWO_B: u8 = 0b1100_0000;
const TAG_THREE_B: u8 = 0b1110_0000;
const TAG_FOUR_B: u8 = 0b1111_0000;
const MAX_ONE_B: u32 = 0x80;
const MAX_TWO_B: u32 = 0x800;
const MAX_THREE_B: u32 = 0x10000;

fn encode_utf8(c: char) -> EncodeUtf8 {
    let code = c as u32;
    let mut buf = [0; 4];
    let pos = if code < MAX_ONE_B {
        buf[3] = code as u8;
        3
    } else if code < MAX_TWO_B {
        buf[2] = (code >> 6 & 0x1F) as u8 | TAG_TWO_B;
        buf[3] = (code & 0x3F) as u8 | TAG_CONT;
        2
    } else if code < MAX_THREE_B {
        buf[1] = (code >> 12 & 0x0F) as u8 | TAG_THREE_B;
        buf[2] = (code >> 6 & 0x3F) as u8 | TAG_CONT;
        buf[3] = (code & 0x3F) as u8 | TAG_CONT;
        1
    } else {
        buf[0] = (code >> 18 & 0x07) as u8 | TAG_FOUR_B;
        buf[1] = (code >> 12 & 0x3F) as u8 | TAG_CONT;
        buf[2] = (code >> 6 & 0x3F) as u8 | TAG_CONT;
        buf[3] = (code & 0x3F) as u8 | TAG_CONT;
        0
    };
    EncodeUtf8 { buf: buf, pos: pos }
}

struct EncodeUtf8 {
    buf: [u8; 4],
    pos: usize,
}

impl EncodeUtf8 {
    fn as_slice(&self) -> &[u8] {
        &self.buf[self.pos..]
    }
}
