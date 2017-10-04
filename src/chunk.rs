use futures::{Async, Stream};
use std::io::Cursor;
use std::mem;
use std::sync::Arc;
use webm::*;

#[derive(Clone)]
pub enum Chunk<B: AsRef<[u8]> = Vec<u8>> {
    Headers {
        bytes: Arc<B>
    },
    ClusterHead {
        keyframe: bool,
        start: u64,
        end: u64,
        // space for a Cluster tag and a Timecode tag
        bytes: [u8;16],
        bytes_used: u8
    },
    ClusterBody {
        bytes: Arc<B>
    }
}

impl<B: AsRef<[u8]>> Chunk<B> {
    pub fn new_cluster_head(timecode: u64) -> Chunk {
        let mut chunk = Chunk::ClusterHead {
            keyframe: false,
            start: 0,
            end: 0,
            bytes: [0;16],
            bytes_used: 0
        };
        chunk.update_timecode(timecode);
        chunk
    }

    pub fn update_timecode(&mut self, timecode: u64) {
        if let &mut Chunk::ClusterHead {ref mut start, ref mut end, ref mut bytes, ref mut bytes_used, ..} = self {
            let delta = *end - *start;
            *start = timecode;
            *end = *start + delta;
            let mut cursor = Cursor::new(bytes as &mut [u8]);
            // buffer is sized so these should never fail
            encode_webm_element(&WebmElement::Cluster, &mut cursor).unwrap();
            encode_webm_element(&WebmElement::Timecode(timecode), &mut cursor).unwrap();
            *bytes_used = cursor.position() as u8;
        }
    }
    pub fn extend_timespan(&mut self, timecode: u64) {
        if let &mut Chunk::ClusterHead {start, ref mut end, ..} = self {
            if timecode > start {
                *end = timecode;
            }
        }
    }
    pub fn mark_keyframe(&mut self, new_keyframe: bool) {
        if let &mut Chunk::ClusterHead {ref mut keyframe, ..} = self {
            *keyframe = new_keyframe;
        }
    }
}

impl<B: AsRef<[u8]>> AsRef<[u8]> for Chunk<B> {
    fn as_ref(&self) -> &[u8] {
        match self {
            &Chunk::Headers {ref bytes, ..} => bytes.as_ref().as_ref(),
            &Chunk::ClusterHead {ref bytes, bytes_used, ..} => bytes[..bytes_used as usize].as_ref(),
            &Chunk::ClusterBody {ref bytes, ..} => bytes.as_ref().as_ref()
        }
    }
}

enum ChunkerState {
    BuildingHeader(Cursor<Vec<u8>>),
    // WIP ClusterHead & body buffer
    BuildingCluster(Chunk, Cursor<Vec<u8>>),
    EmittingClusterBody(Chunk)
}

pub struct WebmChunker<S> {
    stream: S,
    state: ChunkerState
}

impl<'a, S: Stream<Item = WebmElement<'a>>> Stream for WebmChunker<S>
{
    type Item = Chunk;
    type Error = S::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        loop {
            let (return_value, next_state) = match self.state {
                ChunkerState::BuildingHeader(ref mut buffer) => {
                    match self.stream.poll() {
                        Err(passthru) => return Err(passthru),
                        Ok(Async::NotReady) => return Ok(Async::NotReady),
                        Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
                        Ok(Async::Ready(Some(WebmElement::Cluster))) => {
                            let liberated_buffer = mem::replace(buffer, Cursor::new(Vec::new()));
                            let chunk = Chunk::Headers {bytes: Arc::new(liberated_buffer.into_inner())};
                            (
                                Ok(Async::Ready(Some(chunk))),
                                ChunkerState::BuildingCluster(
                                    Chunk::<Vec<u8>>::new_cluster_head(0),
                                    Cursor::new(Vec::new())
                                )
                            )
                        },
                        Ok(Async::Ready(Some(element @ _))) => {
                            encode_webm_element(&element, buffer);
                            continue;
                        }
                    }
                },
                ChunkerState::BuildingCluster(ref mut cluster_head, ref mut buffer) => {
                    return Ok(Async::Ready(None));
                },
                ChunkerState::EmittingClusterBody(ref mut buffer) => {
                    return Ok(Async::Ready(None));
                }
            };

            self.state = next_state;
            return return_value;
        }
    }
}

pub trait WebmStream<T> {
    fn chunk_webm(self) -> WebmChunker<T>;
}

impl<'a, T: Stream<Item = WebmElement<'a>>> WebmStream<T> for T {
    fn chunk_webm(self) -> WebmChunker<T> {
        WebmChunker {
            stream: self,
            state: ChunkerState::BuildingHeader(Cursor::new(Vec::new()))
        }
    }
}

#[cfg(test)]
mod tests {

    use chunk::*;

    #[test]
    fn enough_space_for_header() {
        Chunk::<Vec<u8>>::new_cluster_head(u64::max_value());
    }
}
