use std::net::ToSocketAddrs;
use std::sync::{
    Arc,
    Mutex,
    Weak
};

use bytes::{Bytes, Buf};
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::{
    prelude::*,
    Stream,
    stream::FuturesUnordered,
};
use hyper::{
    Body,
    Response,
    header::{
        CACHE_CONTROL,
        CONTENT_TYPE
    }
};
use stream::iter;
use warp::{
    self,
    Filter,
    path
};
use weak_table::{
    WeakValueHashMap
};
use webmetro::{
    channel::{
        Channel,
        Handle,
        Listener,
        Transmitter
    },
    chunk::WebmStream,
    error::WebmetroError,
    fixers::{
        ChunkStream,
        ChunkTimecodeFixer,
    },
    stream_parser::StreamEbml
, chunk::Chunk};

const BUFFER_LIMIT: usize = 2 * 1024 * 1024;

fn get_stream(channel: Handle) -> impl Stream<Item = Result<Bytes, WebmetroError>> {
    let mut timecode_fixer = ChunkTimecodeFixer::new();
    Listener::new(channel).map(|c| Result::<Chunk, WebmetroError>::Ok(c))
    .map_ok(move |chunk| timecode_fixer.process(chunk))
    .find_starting_point()
    .map_ok(|webm_chunk| iter(webm_chunk).map(Result::<Bytes, WebmetroError>::Ok))
    .try_flatten()
}

fn post_stream(channel: Handle, stream: impl Stream<Item = Result<impl Buf, warp::Error>> + Unpin) -> impl Stream<Item = Result<Bytes, WebmetroError>> {
    let channel = Transmitter::new(channel);
    stream
        .map_err(WebmetroError::from)
        .parse_ebml().with_soft_limit(BUFFER_LIMIT)
        .chunk_webm().with_soft_limit(BUFFER_LIMIT)
        .map_ok(move |chunk| {
            channel.send(chunk);
            Bytes::new()
        })
        .inspect_err(|err| {
            warn!("{}", err)
        })
}

fn media_response(body: Body) -> Response<Body> {
    Response::builder()
        .header(CONTENT_TYPE, "video/webm")
        .header("X-Accel-Buffering", "no")
        .header(CACHE_CONTROL, "no-cache, no-store")
        .body(body)
        .unwrap()
}

pub fn options() -> App<'static, 'static> {
    SubCommand::with_name("relay")
        .about("Hosts an HTTP-based relay server")
        .arg(Arg::with_name("listen")
            .help("The address:port to listen to")
            .required(true))
}

#[tokio::main]
pub async fn run(args: &ArgMatches) -> Result<(), WebmetroError> {
    let channel_map = Arc::new(Mutex::new(WeakValueHashMap::<String, Weak<Mutex<Channel>>>::new()));
    let addr_str = args.value_of("listen").ok_or("Listen address wasn't provided")?;

    let addrs = addr_str.to_socket_addrs()?;
    info!("Binding to {:?}", addrs);
    if addrs.len() == 0 {
        return Err("Listen address didn't resolve".into());
    }

    let channel = path!("live" / String).map(move |name: String| {
        let channel = channel_map.lock().unwrap()
            .entry(name.clone())
            .or_insert_with(|| Channel::new(name.clone()));
        (channel, name)
    });

    let head = channel.clone().and(warp::head())
        .map(|(_, name)| {
            info!("HEAD Request For Channel {}", name);
            media_response(Body::empty())
        });

    let get = channel.clone().and(warp::get())
        .map(|(channel, name)| {
            info!("Listener Connected On Channel {}", name);
            media_response(Body::wrap_stream(get_stream(channel)))
        });

    let post_put = channel.clone().and(warp::post().or(warp::put()).unify())
        .and(warp::body::stream()).map(|(channel, name), stream| {
            info!("Source Connected On Channel {}", name);
            Response::new(Body::wrap_stream(post_stream(channel, stream)))
        });

    let routes = head
        .or(get)
        .or(post_put);

    let mut server_futures: FuturesUnordered<_> = addrs.map(|addr| warp::serve(routes.clone()).try_bind(addr)).collect();

    while let Some(_) = server_futures.next().await {};

    Ok(())
}
