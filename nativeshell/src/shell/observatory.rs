use std::thread;

use crate::util::errno::{errno, set_errno};

fn get_temp_environemt() -> Option<&'static str> {
    #[cfg(target_family = "windows")]
    {
        const TEMP: &str = "TEMP";
        Some(TEMP)
    }
    #[cfg(target_family = "unix")]
    {
        const TMPDIR: &str = "TMPDIR";
        const XDG_RUNTIME_DIR: &str = "XDG_RUNTIME_DIR";
        if std::env::var(XDG_RUNTIME_DIR).is_ok() {
            Some(XDG_RUNTIME_DIR)
        } else if std::env::var(TMPDIR).is_ok() {
            Some(TMPDIR)
        } else {
            None
        }
    }
}

fn have_observatory_url(url: &str, file_suffix: &str) {
    let temp = get_temp_environemt();
    match temp {
        Some(temp) => {
            let dir = std::env::var(temp).unwrap();
            let separator = if dir.ends_with('/') { "" } else { "/" };
            let info = VMServiceInfoFile { uri: url.into() };
            let content = serde_json::to_string_pretty(&info).unwrap();
            let file_name = format!("vmservice.{}", file_suffix);

            println!(
                "nativeshell: Writing VM Service info file into ${{{}}}{}{}",
                temp, separator, file_name,
            );

            let file = format!("{}{}{}", dir, separator, file_name);
            std::fs::write(file, &content).unwrap();
        }
        None => {
            println!("nativeshell: Could not determine temporary folder environment variable.");
            println!("nativeshell: VM Service info file not written.");
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VMServiceInfoFile {
    uri: String,
}

fn dup(fd: libc::c_int) -> libc::c_int {
    unsafe { libc::dup(fd) }
}

fn dup2(src: libc::c_int, dst: libc::c_int) -> libc::c_int {
    loop {
        set_errno(0);
        let res = unsafe { libc::dup2(src, dst) };
        if res == -1 && errno() == libc::EINTR {
            continue;
        }
        return res;
    }
}

#[allow(unused)]
fn _register_observatory_listener(file_suffix: String) {
    const STDOUT_FILENO: i32 = 1;
    let stdout = dup(STDOUT_FILENO);
    let mut pipe = [0; 2];
    unsafe {
        #[cfg(target_family = "windows")]
        libc::pipe(pipe.as_mut_ptr(), STDOUT_FILENO as u32, libc::O_NOINHERIT);

        #[cfg(target_family = "unix")]
        libc::pipe(pipe.as_mut_ptr());

        libc::close(STDOUT_FILENO);
    }
    dup2(pipe[1], STDOUT_FILENO);
    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let mut string = String::new();
        let mut have_url = false;

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

            if have_url {
                continue;
            }

            let utf8 = String::from_utf8_lossy(&buf[0..read as usize]);
            string.push_str(&utf8);

            while let Some(i) = string.find('\n') {
                {
                    let substr = &string[..i];
                    if let Some(url) = substr.strip_prefix(URL_PREFIX) {
                        have_url = true;

                        // after reverting to the original stdout there's no flutter output
                        // anymore; Would be nice to know why this happens;
                        #[cfg(target_family = "windows")]
                        {
                            let file_suffix = file_suffix.clone();
                            let url: String = url.into();
                            thread::spawn(move || {
                                have_observatory_url(&url, &file_suffix);
                            });
                        }

                        #[cfg(target_family = "unix")]
                        {
                            // revert to the original stdout and terminate the thread
                            dup2(stdout, STDOUT_FILENO);
                            have_observatory_url(url, &file_suffix);
                            return;
                        }
                    }
                }
                string.replace_range(..i + 1, "");
            }
        }
    });
}

#[allow(unused_variables)]
pub fn register_observatory_listener(file_suffix: String) {
    #[cfg(any(flutter_profile, debug_assertions))]
    {
        _register_observatory_listener(file_suffix);
    }
}
