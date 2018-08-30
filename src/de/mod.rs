use ::config::Options;

use serde;
use serde::de::IntoDeserializer;
use serde::de::Error as DeError;
use ::{Error, ErrorKind, Result};
use ::internal::SizeLimit;
use self::read::BincodeRead;

pub mod read;
use self::read::SliceReader;

// struct Cursor<'a> {
//     pos: usize,
//     slice: &'a [u8],
// }

// impl<'a> Cursor<'a> {
//     fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
//         if self.pos + buf.len() > self.slice.len() {
//             return Err(ErrorKind::SizeLimit);
//         }
//         buf.copy_from_slice(&self.slice[self.pos..][..buf.len()]);
//         self.pos += buf.len();
//         Ok(())
//     }
// }

/// A Deserializer that reads bytes from a buffer.
///
/// This struct should rarely be used.
/// In most cases, prefer the `deserialize_from` function.
///
/// The ByteOrder that is chosen will impact the endianness that
/// is used to read integers out of the reader.
///
/// ```rust,ignore
/// let d = Deserializer::new(&mut some_reader, SizeLimit::new());
/// serde::Deserialize::deserialize(&mut deserializer);
/// let bytes_read = d.bytes_read();
/// ```
pub(crate) struct Deserializer<'a, O: Options>{
    reader: SliceReader<'a>,
    options: O,
}

impl<'a, 'de, O: Options> Deserializer<'a, O> {
    /// Creates a new Deserializer with a given `Read`er and a size_limit.
    pub(crate) fn new(r: SliceReader<'a>, options: O) -> Deserializer<'a, O> {
        Deserializer {
            reader: r,
            options: options,
        }
    }

    fn read_bytes(&mut self, count: u64) -> Result<()> {
        self.options.limit().add(count)
    }

    fn read_type<T>(&mut self) -> Result<()> {
        use core::mem::size_of;
        self.read_bytes(size_of::<T>() as u64)
    }

    // fn read_vec(&mut self) -> Result<Vec<u8>> {
    //     let len: usize = try!(serde::Deserialize::deserialize(&mut *self));
    //     self.read_bytes(len as u64)?;
    //     self.reader.get_byte_buffer(len)
    // }

    // fn read_string(&mut self) -> Result<String> {
    //     let vec = self.read_vec()?;
    //     String::from_utf8(vec).map_err(|e| ErrorKind::InvalidUtf8Encoding(e.utf8_error()).into())
    // }
}

macro_rules! impl_nums {
    ($ty:ty, $dser_method:ident, $visitor_method:ident, $reader_method:ident) => {
        #[inline]
        fn $dser_method<V>(self, visitor: V) -> Result<V::Value>
            where V: serde::de::Visitor<'de>,
        {
            try!(self.read_type::<$ty>());
            let value = try!(self.reader.$reader_method::<O::Endian>());
            visitor.$visitor_method(value)
        }
    }
}

impl<'de, 'a, O> serde::Deserializer<'de> for &'a mut Deserializer<'de, O>
where
    O: Options,
{
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(ErrorKind::DeserializeAnyNotSupported)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let value: u8 = try!(serde::Deserialize::deserialize(self));
        match value {
            1 => visitor.visit_bool(true),
            0 => visitor.visit_bool(false),
            value => Err(ErrorKind::InvalidBoolEncoding(value).into()),
        }
    }

    impl_nums!(u16, deserialize_u16, visit_u16, read_u16);
    impl_nums!(u32, deserialize_u32, visit_u32, read_u32);
    impl_nums!(u64, deserialize_u64, visit_u64, read_u64);
    impl_nums!(i16, deserialize_i16, visit_i16, read_i16);
    impl_nums!(i32, deserialize_i32, visit_i32, read_i32);
    impl_nums!(i64, deserialize_i64, visit_i64, read_i64);
    impl_nums!(f32, deserialize_f32, visit_f32, read_f32);
    impl_nums!(f64, deserialize_f64, visit_f64, read_f64);

    #[cfg(feature = "i128")]
    impl_nums!(u128, deserialize_u128, visit_u128, read_u128);

    #[cfg(feature = "i128")]
    impl_nums!(i128, deserialize_i128, visit_i128, read_i128);

    serde_if_integer128! {
        #[cfg(not(feature = "i128"))]
        fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
        where
            V: serde::de::Visitor<'de>
        {
            let _ = visitor;
            Err(DeError::custom("u128 is not supported. Enable the `i128` feature of `bincode`"))
        }

        #[cfg(not(feature = "i128"))]
        fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
        where
            V: serde::de::Visitor<'de>
        {
            let _ = visitor;
            Err(DeError::custom("i128 is not supported. Enable the `i128` feature of `bincode`"))
        }
    }

    #[inline]
    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        try!(self.read_type::<u8>());
        if self.reader.slice.is_empty() {
            return Err(ErrorKind::SizeLimit);
        }
        let value = self.reader.slice[0];
        self.reader.slice = &self.reader.slice[1..];
        visitor.visit_u8(value)
    }

    #[inline]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        try!(self.read_type::<i8>());
        if self.reader.slice.is_empty() {
            return Err(ErrorKind::SizeLimit);
        }
        let value = self.reader.slice[0];
        self.reader.slice = &self.reader.slice[1..];
        visitor.visit_i8(value as i8)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        use core::str;

        let error = || ErrorKind::InvalidCharEncoding.into();

        let mut buf = [0u8; 4];

        // Look at the first byte to see how many bytes must be read
        let _ = try!(self.reader.read_exact(&mut buf[..1]));
        let width = utf8_char_width(buf[0]);
        if width == 1 {
            return visitor.visit_char(buf[0] as char);
        }
        if width == 0 {
            return Err(error());
        }

        if self.reader.read_exact(&mut buf[1..width]).is_err() {
            return Err(error());
        }

        let res = try!(
            str::from_utf8(&buf[..width])
                .ok()
                .and_then(|s| s.chars().next())
                .ok_or(error())
        );
        visitor.visit_char(res)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let len: usize = try!(serde::Deserialize::deserialize(&mut *self));
        try!(self.read_bytes(len as u64));
        self.reader.forward_read_str(len, visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let len: usize = try!(serde::Deserialize::deserialize(&mut *self));
        try!(self.read_bytes(len as u64));
        self.reader.forward_read_bytes(len, visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _enum: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        impl<'de, 'a, O> serde::de::EnumAccess<'de> for &'a mut Deserializer<'de, O>
        where O: Options {
            type Error = Error;
            type Variant = Self;

            fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
                where V: serde::de::DeserializeSeed<'de>,
            {
                let idx: u32 = try!(serde::de::Deserialize::deserialize(&mut *self));
                let val: Result<_> = seed.deserialize(idx.into_deserializer());
                Ok((try!(val), self))
            }
        }

        visitor.visit_enum(self)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        struct Access<'a, 'de: 'a, O: Options + 'a> {
            deserializer: &'a mut Deserializer<'de, O>,
            len: usize,
        }

        impl<
            'de,
            'a,
            'b: 'a,
            O: Options,
        > serde::de::SeqAccess<'de> for Access<'a, 'de, O> {
            type Error = Error;

            fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
            where
                T: serde::de::DeserializeSeed<'de>,
            {
                if self.len > 0 {
                    self.len -= 1;
                    let value = try!(serde::de::DeserializeSeed::deserialize(
                        seed,
                        &mut *self.deserializer,
                    ));
                    Ok(Some(value))
                } else {
                    Ok(None)
                }
            }

            fn size_hint(&self) -> Option<usize> {
                Some(self.len)
            }
        }

        visitor.visit_seq(Access {
            deserializer: self,
            len: len,
        })
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let value: u8 = try!(serde::de::Deserialize::deserialize(&mut *self));
        match value {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(&mut *self),
            v => Err(ErrorKind::InvalidTagEncoding(v as usize).into()),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let len = try!(serde::Deserialize::deserialize(&mut *self));

        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        struct Access<'a, 'de: 'a, O: Options + 'a> {
            deserializer: &'a mut Deserializer<'de, O>,
            len: usize,
        }

        impl<
            'de,
            'a,
            'b: 'a,
            O: Options,
        > serde::de::MapAccess<'de> for Access<'a, 'de, O> {
            type Error = Error;

            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
            where
                K: serde::de::DeserializeSeed<'de>,
            {
                if self.len > 0 {
                    self.len -= 1;
                    let key = try!(serde::de::DeserializeSeed::deserialize(
                        seed,
                        &mut *self.deserializer,
                    ));
                    Ok(Some(key))
                } else {
                    Ok(None)
                }
            }

            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
            where
                V: serde::de::DeserializeSeed<'de>,
            {
                let value = try!(serde::de::DeserializeSeed::deserialize(
                    seed,
                    &mut *self.deserializer,
                ));
                Ok(value)
            }

            fn size_hint(&self) -> Option<usize> {
                Some(self.len)
            }
        }

        let len = try!(serde::Deserialize::deserialize(&mut *self));

        visitor.visit_map(Access {
            deserializer: self,
            len: len,
        })
    }

    fn deserialize_struct<V>(
        self,
        _name: &str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_tuple(fields.len(), visitor)
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let message = "Bincode does not support Deserializer::deserialize_identifier";
        Err(Error::custom(message))
    }

    fn deserialize_newtype_struct<V>(self, _name: &str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let message = "Bincode does not support Deserializer::deserialize_ignored_any";
        Err(Error::custom(message))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

impl<'de, 'a, O> serde::de::VariantAccess<'de> for &'a mut Deserializer<'de, O>
where O: Options{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
        where T: serde::de::DeserializeSeed<'de>,
    {
        serde::de::DeserializeSeed::deserialize(seed, self)
    }

    fn tuple_variant<V>(self,
                      len: usize,
                      visitor: V) -> Result<V::Value>
        where V: serde::de::Visitor<'de>,
    {
        serde::de::Deserializer::deserialize_tuple(self, len, visitor)
    }

    fn struct_variant<V>(self,
                       fields: &'static [&'static str],
                       visitor: V) -> Result<V::Value>
        where V: serde::de::Visitor<'de>,
    {
        serde::de::Deserializer::deserialize_tuple(self, fields.len(), visitor)
    }
}
static UTF8_CHAR_WIDTH: [u8; 256] = [
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x1F
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x3F
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x5F
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1, // 0x7F
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 0x9F
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0, // 0xBF
0,0,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2, // 0xDF
3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3, // 0xEF
4,4,4,4,4,0,0,0,0,0,0,0,0,0,0,0, // 0xFF
];

// This function is a copy of core::str::utf8_char_width
fn utf8_char_width(b: u8) -> usize {
    UTF8_CHAR_WIDTH[b as usize] as usize
}
