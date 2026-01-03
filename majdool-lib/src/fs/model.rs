use pin_project::pin_project;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct Size {
    inner: usize,
    complete: bool,
}

impl Default for Size {
    fn default() -> Self {
        Self {
            inner: 0,
            complete: false,
        }
    }
}

impl Size {
    fn add(&mut self, value: usize) -> Result<(), ()> {
        if self.complete {
            Err(())
        } else {
            self.inner += value;
            Ok(())
        }
    }

    fn finalize(&mut self) -> Result<(), ()> {
        if self.complete {
            Err(())
        } else {
            self.complete = true;
            Ok(())
        }
    }
}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
#[pin_project]
pub struct StreamComparator {
    // Receivers stream bytes into the comparator.
    left_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    right_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    // Buffers hold the physical stream representation.
    left_buffer: Vec<u8>,
    right_buffer: Vec<u8>,
    // Sizes track the virtual stream size and completion.
    left_size: Size,
    right_size: Size,
    // Tracks the absolute position across the streams.
    // So this indexes into the virtual streams, which may be have a different physical representation based off `truncated`.
    cursor: usize,
}

impl StreamComparator {
    pub fn new(
        left_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
        right_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            left_rx,
            right_rx,
            left_buffer: Vec::default(),
            right_buffer: Vec::default(),
            left_size: Size::default(),
            right_size: Size::default(),
            cursor: 0,
        }
    }
}

impl Future for StreamComparator {
    type Output = Result<bool, ()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let mut is_pending = false;

            if !self.left_size.complete {
                match self.left_rx.poll_recv(cx) {
                    Poll::Ready(Some(data)) => {
                        self.left_size.add(data.len())?;
                        self.left_buffer.extend(data);
                    }
                    Poll::Ready(None) => {
                        self.left_size.finalize()?;
                    }
                    Poll::Pending => {
                        is_pending = true;
                    }
                }
            }

            if !self.right_size.complete {
                match self.right_rx.poll_recv(cx) {
                    Poll::Ready(Some(data)) => {
                        self.right_size.add(data.len())?;
                        self.right_buffer.extend(data);
                    }
                    Poll::Ready(None) => {
                        self.right_size.finalize()?;
                    }
                    Poll::Pending => {
                        is_pending = true;
                    }
                }
            }

            let absolute_head = std::cmp::min(self.left_size.inner, self.right_size.inner);

            if self.cursor < absolute_head {
                /* Our state looks something like this:
                 *  left_stream  = |xxxxx|.............|
                 *  right_stream = |xxxxx|....................|
                 *  left_buffer  =       |.............|
                 *  right_buffer =       |....................|
                 *  absolute_head  <------------------->
                 *  cursor         <----->
                 *  relative_head        <------------->
                 *  left_remain  =                     ||
                 *  right_remain =                     |......|
                 */

                let relative_head = absolute_head - self.cursor;
                let left_remaining = self.left_buffer.split_off(relative_head);
                let right_remaining = self.right_buffer.split_off(relative_head);

                if self.left_buffer != self.right_buffer {
                    return Poll::Ready(Ok(false));
                }

                self.left_buffer = left_remaining;
                self.right_buffer = right_remaining;

                self.cursor = absolute_head;
            }

            if self.left_size.complete && self.left_size.inner < self.right_size.inner {
                return Poll::Ready(Ok(false));
            }

            if self.right_size.complete && self.right_size.inner < self.left_size.inner {
                return Poll::Ready(Ok(false));
            }

            if self.left_size.complete
                && self.right_size.complete
                && self.cursor == self.left_size.inner
                && self.cursor == self.right_size.inner
            {
                return Poll::Ready(Ok(true));
            }

            if is_pending {
                return Poll::Pending;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rand::RngCore;
    use std::iter::Cloned;
    use tokio::sync::mpsc;

    async fn run_comparison(
        left_chunks: Vec<Vec<u8>>,
        right_chunks: Vec<Vec<u8>>,
    ) -> Result<bool, ()> {
        let (left_tx, left_rx) = mpsc::channel(10);
        let (right_tx, right_rx) = mpsc::channel(10);

        let comparitor = StreamComparator::new(left_rx, right_rx);
        let send_left = tokio::spawn(async move {
            for chunk in left_chunks {
                let _ = left_tx.send(chunk).await;
            }
        });
        let send_right = tokio::spawn(async move {
            for chunk in right_chunks {
                let _ = right_tx.send(chunk).await;
            }
        });
        let (_, _, result) = tokio::join!(send_left, send_right, comparitor);
        result
    }

    #[test]
    fn size() {
        let mut size = Size::default();
        assert_eq!(size.inner, 0);
        assert!(!size.complete);

        size.add(1).unwrap();
        assert_eq!(size.inner, 1);
        assert!(!size.complete);

        size.add(100).unwrap();
        assert_eq!(size.inner, 101);
        assert!(!size.complete);

        size.finalize().unwrap();
        assert_eq!(size.inner, 101);
        assert!(size.complete);

        size.finalize().unwrap_err();
        size.add(1).unwrap_err();
        assert_eq!(size.inner, 101);
        assert!(size.complete);
    }

    #[tokio::test]
    async fn stream_comparator_empty() {
        let result = run_comparison(vec![], vec![]).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn stream_comparator_matching() {
        let data = vec![1, 2, 3, 4, 5];
        let result = run_comparison(vec![data.clone()], vec![data.clone()])
            .await
            .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn stream_comparator_matching_multiple_chunks() {
        let result = run_comparison(
            vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]],
            vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]],
        )
        .await
        .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn stream_comparator_matching_different_chunks() {
        let result = run_comparison(
            vec![vec![1, 2, 3, 4, 5], vec![6, 7, 8], vec![9, 10]],
            vec![vec![1, 2], vec![3, 4, 5], vec![6, 7, 8, 9, 10]],
        )
        .await
        .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn stream_comparator_matching_varying_chunks() {
        let large = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let result = run_comparison(
            vec![large.clone()],
            vec![
                vec![1],
                vec![2],
                vec![3],
                vec![4],
                vec![5],
                vec![6],
                vec![7],
                vec![8],
                vec![9],
                vec![10],
            ],
        )
        .await
        .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn stream_comparator_mismatching() {
        let result = run_comparison(vec![vec![1, 2, 3, 4, 5]], vec![vec![1, 2, 3, 4, 6]])
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn stream_comparator_early_mismatching() {
        let result = run_comparison(
            vec![vec![1, 2, 3], vec![4, 5, 6]],
            vec![vec![1, 2, 4], vec![4, 5, 6]],
        )
        .await
        .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn stream_comparator_late_mismatching() {
        let result = run_comparison(
            vec![vec![1, 2, 3], vec![4, 5, 6]],
            vec![vec![1, 2, 3], vec![4, 5, 7]],
        )
        .await
        .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn stream_comparator_left_longer() {
        let result = run_comparison(vec![vec![1, 2, 3, 4, 5]], vec![vec![1, 2, 3]])
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn stream_comparator_right_longer() {
        let result = run_comparison(vec![vec![1, 2, 3]], vec![vec![1, 2, 3, 4, 5]])
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn stream_comparator_left_longer_multiple_chunks() {
        let result = run_comparison(vec![vec![1, 2, 3], vec![4, 5]], vec![vec![1, 2, 3]])
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn stream_comparator_mismatching_empty() {
        let result = run_comparison(vec![vec![1, 2, 3]], vec![]).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn stream_comparator_empty_chunks() {
        let result = run_comparison(
            vec![vec![], vec![1, 2, 3], vec![]],
            vec![vec![1], vec![], vec![2, 3]],
        )
        .await
        .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn stream_comparator_matching_large_dataset() {
        let large_data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let result = run_comparison(vec![large_data.clone()], vec![large_data.clone()])
            .await
            .unwrap();
        assert!(result);
    }

    proptest! {
        #[test]
        fn stream_comparator_matching_proptest(i in 1usize..10_000, j in 1usize..10_000) {
            let mut rng = rand::thread_rng();
            let mut data = vec![0u8; 100_000];
            rng.fill_bytes(&mut data);
            let left_chunks: Vec<Vec<u8>> = data.chunks(i).map(|c| c.to_vec()).collect();
            let right_chunks: Vec<Vec<u8>> = data.chunks(j).map(|c| c.to_vec()).collect();

            tokio_test::block_on(async {
                let result = run_comparison(left_chunks, right_chunks).await.unwrap();
                assert!(result);
            });
        }

        #[test]
        fn stream_comparator_mismatching_proptest(i in 1usize..10_000, j in 1usize..10_000, k in 0usize..100_000) {
            let mut rng = rand::thread_rng();
            let mut data = vec![0u8; 100_000];
            rng.fill_bytes(&mut data);
            let left_chunks: Vec<Vec<u8>> = data.chunks(i).map(|c| c.to_vec()).collect();
            // Mutate the element at k.
            data[k] = !data[k];
            let right_chunks: Vec<Vec<u8>> = data.chunks(j).map(|c| c.to_vec()).collect();

            tokio_test::block_on(async {
                let result = run_comparison(left_chunks, right_chunks).await.unwrap();
                assert!(!result);
            });
        }
    }
}
