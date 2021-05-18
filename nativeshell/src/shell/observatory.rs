use std::thread;

fn have_observatory_url(url: &str) {
    eprintln!("Observatory URL '{}'", url);
}

// TODO(knopp): handle possible eintr in dup and dup2
pub fn register_observatory_listener() {
    let stdout = unsafe { libc::dup(1) };
    let mut pipe = [0; 2];
    unsafe {
        #[cfg(target_family = "windows")]
        libc::pipe(pipe.as_mut_ptr(), 1, libc::O_NOINHERIT);

        #[cfg(target_family = "unix")]
        libc::pipe(pipe.as_mut_ptr());

        libc::close(1);
        libc::dup2(pipe[1], 1);
    }
    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let mut string = String::new();
        let mut seen_observatory_url = false;
        const URL_PREFIX: &str = "flutter: Observatory listening on ";
        loop {
            let read = unsafe {
                #[cfg(target_family = "windows")]
                let read = libc::read(pipe[0], buf.as_mut_ptr() as *mut _, buf.len() as u32);

                #[cfg(target_family = "unix")]
                let read = libc::read(pipe[0], buf.as_mut_ptr() as *mut _, buf.len());

                if read < 0 {
                    panic!("Could not read from stdout");
                }

                #[cfg(target_family = "windows")]
                libc::write(stdout, buf.as_ptr() as *const _, read as u32);

                #[cfg(target_family = "unix")]
                libc::write(stdout, buf.as_ptr() as *const _, read as usize);
                read
            };

            if !seen_observatory_url {
                let utf8 = String::from_utf8_lossy(&buf[0..read as usize]);
                string.push_str(&utf8);

                loop {
                    if let Some(i) = string.find('\n') {
                        {
                            let substr = &string[..i];
                            if substr.starts_with(URL_PREFIX) {
                                seen_observatory_url = true;
                                have_observatory_url(&substr[URL_PREFIX.len()..]);
                            }
                        }
                        string.replace_range(..i + 1, "");
                    } else {
                        break;
                    }
                }
            }
        }
    });
}
