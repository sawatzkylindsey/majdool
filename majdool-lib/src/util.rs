use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub(crate) async fn compute_file_hash(path: impl AsRef<Path>) -> Result<[u8; 32], std::io::Error> {
    let file = File::open(path).await?;
    compute_hash(file).await
}

async fn compute_hash<R: AsyncRead + Unpin>(mut reader: R) -> Result<[u8; 32], std::io::Error> {
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().into())
}

async fn copy_file(
    source: impl AsRef<Path>,
    target: impl AsRef<Path>,
) -> Result<u64, std::io::Error> {
    let mut src = File::open(source).await?;
    let mut tgt = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .await?;

    let mut buffer = [0; 8192];
    let mut total = 0;

    loop {
        let n = src.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        tgt.write_all(&buffer[..n]).await?;
        total += n as u64;
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::tempdir;

    #[tokio::test]
    async fn empty_input() {
        let hash = compute_hash(Cursor::new(b"")).await.unwrap();
        let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(hex::encode(hash), expected);
    }

    #[tokio::test]
    async fn hello_world() {
        let hash = compute_hash(Cursor::new(b"hello world")).await.unwrap();
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert_eq!(hex::encode(hash), expected);
    }

    #[tokio::test]
    async fn large_input() {
        let data = vec![0u8; 100_000];
        let hash = compute_hash(Cursor::new(data)).await.unwrap();
        let expected = "9192c25b734fcbadbe32dadc28089c60db0e39f90cc20ce2e5733f57261acc0c";
        assert_eq!(hex::encode(hash), expected);
    }

    #[tokio::test]
    async fn small_chunks_match_normal_read() {
        let data = b"the quick brown fox jumps over the lazy dog";
        let hash1 = compute_hash(Cursor::new(data)).await.unwrap();

        let chunked = ChunkedReader {
            inner: Cursor::new(data),
            chunk_size: 3,
        };
        let hash2 = compute_hash(chunked).await.unwrap();

        assert_eq!(hex::encode(hash1), hex::encode(hash2));
    }

    #[tokio::test]
    async fn copies_file_contents() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dst = dir.path().join("dst.txt");

        tokio::fs::write(&src, b"hello world").await.unwrap();
        let bytes = copy_file(&src, &dst).await.unwrap();

        assert_eq!(tokio::fs::read(&dst).await.unwrap(), b"hello world");
    }

    #[tokio::test]
    async fn fails_if_dest_exists() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.txt");
        let dst = dir.path().join("dst.txt");

        tokio::fs::write(&src, b"hello").await.unwrap();
        tokio::fs::write(&dst, b"existing").await.unwrap();

        let err = copy_file(&src, &dst).await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    }

    #[tokio::test]
    async fn fails_if_src_missing() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("missing.txt");
        let dst = dir.path().join("dst.txt");

        let err = copy_file(&src, &dst).await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    struct ChunkedReader<R> {
        inner: R,
        chunk_size: usize,
    }

    impl<R: AsyncRead + Unpin> AsyncRead for ChunkedReader<R> {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            let max = buf.remaining().min(self.chunk_size);
            let mut temp = vec![0u8; max];
            let mut temp_buf = tokio::io::ReadBuf::new(&mut temp);

            match std::pin::Pin::new(&mut self.inner).poll_read(cx, &mut temp_buf) {
                std::task::Poll::Ready(Ok(())) => {
                    buf.put_slice(temp_buf.filled());
                    std::task::Poll::Ready(Ok(()))
                }
                other => other,
            }
        }
    }
}
