//! Steam's binary KeyValues (achievement stats, appinfo.vdf, packageinfo.vdf): 0x00 object, 0x01 string,
//! 0x02 int32, 0x03 float, 0x07 uint64, 0x0A int64, 0x08 end. Keys are lowercased, like the Electron host.

use serde_json::{json, Map, Value};

pub struct Reader<'a> { pub b: &'a [u8], pub i: usize }

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Self { Reader { b, i: 0 } }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> { let s = self.b.get(self.i..self.i + n)?; self.i += n; Some(s) }
    pub fn u32(&mut self) -> Option<u32> { Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?)) }
    pub fn i32(&mut self) -> Option<i32> { Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?)) }
    pub fn u64(&mut self) -> Option<u64> { Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?)) }
    pub fn i64(&mut self) -> Option<i64> { Some(i64::from_le_bytes(self.take(8)?.try_into().ok()?)) }
    pub fn f32(&mut self) -> Option<f32> { Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?)) }
    pub fn cstr(&mut self) -> Option<String> {
        let end = self.b[self.i..].iter().position(|c| *c == 0)? + self.i;
        let s = String::from_utf8_lossy(&self.b[self.i..end]).into_owned();
        self.i = end + 1;
        Some(s)
    }
}

/// One object; `keys` is appinfo's key table (newer versions store keys as indexes into it).
pub fn read(r: &mut Reader, keys: Option<&[String]>) -> Option<Value> {
    let mut d = Map::new();
    loop {
        let t = *r.b.get(r.i)?; r.i += 1;
        if t == 0x08 { return Some(Value::Object(d)); }
        let key = match keys { Some(k) => { let n = r.i32()?; k.get(n as usize).cloned().unwrap_or_else(|| n.to_string()) } None => r.cstr()? };
        let v = match t {
            0x00 => read(r, keys)?,
            0x01 => json!(r.cstr()?),
            0x02 => json!(r.i32()?),
            0x03 => json!(r.f32()?),
            0x07 => json!(r.u64()?.to_string()),
            0x0a => json!(r.i64()?),
            _ => return None,
        };
        d.insert(key.to_lowercase(), v);
    }
}

pub fn load(path: &std::path::Path) -> Option<Value> { let b = std::fs::read(path).ok()?; read(&mut Reader::new(&b), None) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_objects() {
        let mut b = vec![0x00]; b.extend(b"620\0");
        b.push(0x01); b.extend(b"Name\0Portal 2\0");
        b.push(0x02); b.extend(b"data\0"); b.extend(5i32.to_le_bytes());
        b.push(0x08); b.push(0x08);
        assert_eq!(read(&mut Reader::new(&b), None).unwrap(), json!({"620": {"name": "Portal 2", "data": 5}}));
        assert!(read(&mut Reader::new(&[0x01, b'x']), None).is_none());
    }
}
