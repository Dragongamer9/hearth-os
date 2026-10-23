//! Where a live stream waits for its viewers: the newest picture (FrameHub) or the newest stretch of sound
//! (ByteHub). Each viewer takes what's newer than what it had; a slow viewer skips pictures instead of falling behind.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Default)]
struct Latest { seq: u64, frame: Option<Arc<Vec<u8>>>, closed: bool }

#[derive(Default)]
pub struct FrameHub { m: Mutex<Latest>, cv: Condvar }

impl FrameHub {
    pub fn new() -> Arc<FrameHub> { Arc::new(FrameHub::default()) }
    pub fn publish(&self, f: Vec<u8>) {
        let mut l = self.m.lock().unwrap();
        l.seq += 1;
        l.frame = Some(Arc::new(f));
        l.closed = false;
        self.cv.notify_all();
    }
    /// No more pictures for now (the camera or the capture stopped): waiting viewers are let go.
    pub fn close(&self) {
        let mut l = self.m.lock().unwrap();
        l.closed = true;
        self.cv.notify_all();
    }
    pub fn latest(&self) -> Option<(u64, Arc<Vec<u8>>)> {
        let l = self.m.lock().unwrap();
        l.frame.clone().map(|f| (l.seq, f))
    }
    pub fn seq(&self) -> u64 { self.m.lock().unwrap().seq }
    /// The newest picture after `seq`, waiting up to `timeout` for one.
    pub fn wait_newer(&self, seq: u64, timeout: Duration) -> Option<(u64, Arc<Vec<u8>>)> {
        let l = self.m.lock().unwrap();
        let (l, _) = self.cv.wait_timeout_while(l, timeout, |l| l.seq <= seq && !l.closed).unwrap();
        if l.seq > seq { l.frame.clone().map(|f| (l.seq, f)) } else { None }
    }
}

#[derive(Default)]
struct Chunks { first: u64, q: VecDeque<Arc<Vec<u8>>> }

/// A live byte stream (MP3 sound) for many listeners: the last few chunks are kept, a new listener starts at the newest.
#[derive(Default)]
pub struct ByteHub { m: Mutex<Chunks>, cv: Condvar }

impl ByteHub {
    pub fn new() -> Arc<ByteHub> { Arc::new(ByteHub::default()) }
    pub fn push(&self, b: Vec<u8>) {
        let mut c = self.m.lock().unwrap();
        c.q.push_back(Arc::new(b));
        while c.q.len() > 64 { c.q.pop_front(); c.first += 1; }
        self.cv.notify_all();
    }
    /// The number of the next chunk to come.
    pub fn next(&self) -> u64 { let c = self.m.lock().unwrap(); c.first + c.q.len() as u64 }
    /// Chunk number `n` (or the oldest kept, if it's gone), waiting for it up to `timeout`. Returns it and the number after it.
    pub fn get(&self, n: u64, timeout: Duration) -> Option<(Arc<Vec<u8>>, u64)> {
        let c = self.m.lock().unwrap();
        let (c, _) = self.cv.wait_timeout_while(c, timeout, |c| c.first + c.q.len() as u64 <= n).unwrap();
        let n = n.max(c.first);
        let i = (n - c.first) as usize;
        c.q.get(i).map(|b| (b.clone(), n + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames() {
        let h = FrameHub::new();
        assert!(h.latest().is_none());
        assert!(h.wait_newer(0, Duration::from_millis(20)).is_none());
        let h2 = h.clone();
        let t = std::thread::spawn(move || h2.wait_newer(0, Duration::from_secs(5)));
        std::thread::sleep(Duration::from_millis(50));
        h.publish(vec![1]);
        assert_eq!(*t.join().unwrap().unwrap().1, vec![1]);
        h.publish(vec![2]);
        h.publish(vec![3]);
        let (s, f) = h.wait_newer(1, Duration::from_millis(10)).unwrap();
        assert_eq!((s, f.to_vec()), (3, vec![3]));
        h.close();
        assert!(h.wait_newer(3, Duration::from_secs(5)).is_none());
    }
    #[test]
    fn bytes() {
        let h = ByteHub::new();
        assert_eq!(h.next(), 0);
        for i in 0..70u8 { h.push(vec![i]); }
        assert_eq!(h.next(), 70);
        let (b, n) = h.get(0, Duration::from_millis(10)).unwrap();
        assert_eq!((b[0], n), (6, 7));
        assert!(h.get(70, Duration::from_millis(10)).is_none());
    }
}
