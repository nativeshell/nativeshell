use std::thread;

use crate::util::errno::{errno, set_errno};

fn have_observatory_url(url: &str, file_suffix: &str) {
    #[cfg(target_family = "windows")]
    const TMP_ENV: &str = "TEMP";

    #[cfg(target_family = "unix")]
    const TMP_ENV: &str = "TMPDIR";

    let info = VMServiceInfoFile { uri: url.into() };
    let content = serde_json::to_string_pretty(&info).unwrap();
    let file_name = format!("vmservice.{}", file_suffix);
    let tmp_dir = std::env::var(TMP_ENV).unwrap_or("/tmp".into());
    println!(
        "nativeshell: Writing VM Service info file ${{{}}}{}",
        TMP_ENV, file_name,
    );

    let file = format!("{}{}", tmp_dir, file_name);
    std::fs::write(file, &content).unwrap();
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
    let stdout = dup(libc::STDOUT_FILENO);
    let mut pipe = [0; 2];
    unsafe {
        #[cfg(target_family = "windows")]
        libc::pipe(pipe.as_mut_ptr(), libc::STDOUT_FILENO, libc::O_NOINHERIT);

        #[cfg(target_family = "unix")]
        libc::pipe(pipe.as_mut_ptr());

        libc::close(libc::STDOUT_FILENO);
    }
    dup2(pipe[1], libc::STDOUT_FILENO);
    thread::spawn(move || {
        let mut buf = [0u8; 1024];
        let mut string = String::new();

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

            let utf8 = String::from_utf8_lossy(&buf[0..read as usize]);
            string.push_str(&utf8);

            loop {
                if let Some(i) = string.find('\n') {
                    {
                        let substr = &string[..i];
                        if substr.starts_with(URL_PREFIX) {
                            // revert to the original stdout
                            dup2(stdout, libc::STDOUT_FILENO);

                            have_observatory_url(&substr[URL_PREFIX.len()..], &file_suffix);
                            return;
                        }
                    }
                    string.replace_range(..i + 1, "");
                } else {
                    break;
                }
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
