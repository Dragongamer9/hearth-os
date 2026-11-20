//! Waits for a capture card to be plugged in or out: inotify on /dev for video devices appearing, going and getting
//! their permissions (udev sets them just after), sleeping in the kernel in between: nothing polls ("Looking for a
//! capture card…" uses nothing). CI's stand-in card: the folder its JSON file is in.

use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::Stream;

/// One () each time cards may have come or gone (settled for a moment, so udev has finished with them).
pub fn changes() -> impl Stream<Item = ()> {
    let (tx, rx) = unbounded();
    let _ = std::thread::Builder::new().name("cc-watch".into()).stack_size(64 * 1024).spawn(move || reader(tx));
    rx
}

fn reader(tx: UnboundedSender<()>) {
    let (dir, only) = match crate::card::fake_file() {
        Some(f) => (f.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| "/tmp".into()), f.file_name().map(|n| n.to_string_lossy().to_string())),
        None => ("/dev".into(), None),
    };
    let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_encoded_bytes()) else { return };
    // SAFETY: inotify on a path we own the string of; the descriptor is this thread's.
    let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
    if fd < 0 {
        return;
    }
    let mask = libc::IN_CREATE | libc::IN_DELETE | libc::IN_ATTRIB | libc::IN_MOVED_TO | libc::IN_CLOSE_WRITE;
    if unsafe { libc::inotify_add_watch(fd, c.as_ptr(), mask) } < 0 {
        unsafe { libc::close(fd) };
        return;
    }
    let mut buf = [0u8; 4096];
    loop {
        // Blocks until something happens in the folder.
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            break;
        }
        if !names(&buf[..n as usize]).iter().any(|nm| match &only {
            Some(o) => nm == o,
            None => nm.starts_with("video"),
        }) {
            continue;
        }
        // Let udev finish (permissions, the second node), then drain what came meanwhile.
        std::thread::sleep(std::time::Duration::from_millis(600));
        let mut pfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
        while unsafe { libc::poll(&mut pfd, 1, 0) } > 0 {
            if unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) } <= 0 {
                break;
            }
        }
        if tx.unbounded_send(()).is_err() {
            break;
        }
    }
    unsafe { libc::close(fd) };
}

/// The names in a batch of inotify events.
pub fn names(b: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 16 <= b.len() {
        let len = u32::from_ne_bytes([b[i + 12], b[i + 13], b[i + 14], b[i + 15]]) as usize;
        let name = &b[i + 16..(i + 16 + len).min(b.len())];
        out.push(String::from_utf8_lossy(&name[..name.iter().position(|x| *x == 0).unwrap_or(name.len())]).to_string());
        i += 16 + len;
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_inotify_names() {
        let mut b = vec![0u8; 16];
        b[12] = 8;
        b.extend_from_slice(b"video2\0\0");
        let mut c = vec![0u8; 16];
        c[12] = 4;
        c.extend_from_slice(b"sda\0");
        b.extend(c);
        assert_eq!(super::names(&b), vec!["video2".to_string(), "sda".to_string()]);
    }
}
