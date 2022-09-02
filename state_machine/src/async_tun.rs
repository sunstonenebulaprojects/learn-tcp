use async_trait::async_trait;
use std::io;
use std::os::unix::prelude::AsRawFd;
use tokio::io::unix::AsyncFd;

#[async_trait]
pub trait AsyncTun {
    async fn read(&self, out: &mut [u8]) -> io::Result<usize>;
    async fn write(&self, buf: &[u8]) -> io::Result<usize>;
}

#[derive(Debug)]
pub struct AsyncIFace {
    inner: AsyncFd<tun_tap::Iface>,
    flags: i32,
}

impl AsyncIFace {
    pub fn new(nic: tun_tap::Iface) -> io::Result<Self> {
        let fd = nic.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL, 0) };
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
        if flags == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Self {
                inner: AsyncFd::new(nic)?,
                flags,
            })
        }
    }
}

#[async_trait]
impl AsyncTun for AsyncIFace {
    async fn read(&self, out: &mut [u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.readable().await?;

            match guard.try_io(|inner| inner.get_ref().recv(out)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }

    async fn write(&self, buf: &[u8]) -> io::Result<usize> {
        loop {
            let mut guard = self.inner.writable().await?;

            match guard.try_io(|inner| inner.get_ref().send(buf)) {
                Ok(result) => return result,
                Err(_would_block) => continue,
            }
        }
    }
}

impl Drop for AsyncIFace {
    fn drop(&mut self) {
        let fd = self.inner.as_raw_fd();
        unsafe { libc::fcntl(fd, libc::F_SETFL, self.flags) };
    }
}
