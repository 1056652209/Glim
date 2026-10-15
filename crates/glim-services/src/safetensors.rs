//! Stream header metadata to a pageable report without reading tensor payloads.
use crate::{paged::TextPage, report::Report, work::Cancellation};
use anyhow::{Context, Result, ensure};
use serde::de::{DeserializeSeed, Error, MapAccess, Visitor};
use std::{
    collections::BTreeMap,
    fmt::{self, Write as _},
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

pub fn read_preview(path: &Path) -> Result<TextPage> {
    read_preview_cancellable(path, &Cancellation::default())
}
pub fn read_preview_cancellable(path: &Path, cancel: &Cancellation) -> Result<TextPage> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut prefix = [0; 8];
    file.read_exact(&mut prefix)
        .context("Missing safetensors header length")?;
    let length = u64::from_le_bytes(prefix);
    ensure!(
        length <= size.saturating_sub(8),
        "Truncated safetensors header"
    );
    let payload = size - 8 - length;
    let mut text = Report::new()?;
    writeln!(
        text,
        "SAFETENSORS\n\nFile size       {size} bytes\nHeader          {length} bytes\nTensor payload  {payload} bytes\n\nTENSORS AND METADATA"
    )?;
    let input = crate::work::Reader {
        inner: file.take(length),
        cancel: cancel.clone(),
    };
    let mut decoder = serde_json::Deserializer::from_reader(BufReader::new(input));
    let mut header = Header {
        text: &mut text,
        payload,
        count: 0,
        parameters: 0,
        types: BTreeMap::new(),
        cancel,
    };
    (&mut header).deserialize(&mut decoder)?;
    decoder.end()?;
    let count = header.count;
    let parameters = header.parameters;
    let types = std::mem::take(&mut header.types);
    drop(header);
    writeln!(
        text,
        "\nTensors         {count}\nParameters      {parameters}\n\nPRECISION"
    )?;
    for (dtype, count) in types {
        writeln!(text, "{dtype:<12} {count} tensors")?;
    }
    text.push_str("\nHeader inspection only; weights were not loaded. Offsets are range-checked; this is not full model validation.\n");
    text.finish(path)
}

struct Header<'a> {
    text: &'a mut Report,
    payload: u64,
    count: u64,
    parameters: u128,
    types: BTreeMap<String, u64>,
    cancel: &'a Cancellation,
}
impl<'de> DeserializeSeed<'de> for &mut Header<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        decoder: D,
    ) -> std::result::Result<(), D::Error> {
        decoder.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for &mut Header<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a safetensors header object")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> std::result::Result<(), M::Error> {
        while let Some(name) = map.next_key::<String>()? {
            self.cancel.check().map_err(M::Error::custom)?;
            if name == "__metadata__" {
                map.next_value_seed(Metadata(self.text))?;
                continue;
            }
            #[derive(serde::Deserialize)]
            struct Tensor {
                dtype: String,
                shape: Vec<u64>,
                data_offsets: [u64; 2],
            }
            let tensor: Tensor = map.next_value()?;
            let [begin, end] = tensor.data_offsets;
            if begin > end || end > self.payload {
                return Err(M::Error::custom("Tensor extends beyond the file"));
            }
            let elements = tensor
                .shape
                .iter()
                .try_fold(1u128, |n, dim| n.checked_mul(u128::from(*dim)))
                .ok_or_else(|| M::Error::custom("Tensor shape overflow"))?;
            self.parameters = self
                .parameters
                .checked_add(elements)
                .ok_or_else(|| M::Error::custom("Parameter count overflow"))?;
            self.count += 1;
            *self.types.entry(tensor.dtype.clone()).or_default() += 1;
            write!(self.text, "{name}  ·  {}  ·  ", tensor.dtype).map_err(M::Error::custom)?;
            if tensor.shape.is_empty() {
                self.text.push_str("[] (scalar)");
            }
            for (index, dimension) in tensor.shape.iter().enumerate() {
                if index > 0 {
                    self.text.push_str(" × ");
                }
                write!(self.text, "{dimension}").map_err(M::Error::custom)?;
            }
            writeln!(self.text, "  ·  {} bytes", end - begin).map_err(M::Error::custom)?;
        }
        Ok(())
    }
}
struct Metadata<'a>(&'a mut Report);
impl<'de> DeserializeSeed<'de> for Metadata<'_> {
    type Value = ();
    fn deserialize<D: serde::Deserializer<'de>>(
        self,
        decoder: D,
    ) -> std::result::Result<(), D::Error> {
        decoder.deserialize_map(self)
    }
}
impl<'de> Visitor<'de> for Metadata<'_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("string metadata")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> std::result::Result<(), M::Error> {
        self.0.push_str("\nMETADATA\n");
        while let Some((key, value)) = map.next_entry::<String, String>()? {
            writeln!(self.0, "{key}: {value}").map_err(M::Error::custom)?;
        }
        Ok(())
    }
}
