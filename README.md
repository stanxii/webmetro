# webmetro

`webmetro` is a simple relay server for broadcasting a WebM stream from one uploader to many downloaders, via HTTP.

The initialization segment is remembered, so that viewers can join mid-stream.

Cluster timestamps are rewritten to be monotonic, so multiple (compatibly-encoded) webm files can be chained together without clients needing to reconnect.

## Usage

Launch a relay server with the `relay` subcommand:

`webmetro relay localhost:8080`

At this point you can open http://localhost:8080/live in a web browser.

Next, a source client will need to `POST` or `PUT` a stream to that URL; a static file can be uploaded with the `send` subcommand:

`webmetro send --throttle http://localhost:8080/live < file.webm`

You can even glue together multiple files, provided they share the same codecs and track order:

`cat 1.webm 2.webm 3.webm | webmetro send --throttle http://localhost:8080/live`

You can use ffmpeg to transcode a non-WebM file or access a media device:

`ffmpeg -i file.mp4 -deadline realtime -threads 4 -vb 700k -vcodec libvpx -f webm -live 1 - | webmetro send --throttle http://localhost:8080/live`

(if the source is itself a live stream, you can leave off the `--throttle` flag)

## Limitations

* HTTPS is not supported yet. It really should be.
* There aren't any access controls on either the source or viewer roles yet.
* Currently the server only recognizes a single stream, at `/live`.
* The server tries to start a viewer at a cluster containing a keyframe; it is not yet smart enough to ensure that the keyframe belongs to the *video* stream.
* The server doesn't parse any metadata, such as tags; the Info segment is stripped out, everything else is blindly passed along.
* The server drops any source that it feels uses too much buffer space. This is not yet configurable, though sane files probably won't hit the limit. (Essentially, clusters & the initialization segment can't individually be more than 2M)

## See Also

* the [Icecast](http://www.icecast.org/) streaming server likewise relays media streams over HTTP, and supports additional non-WebM formats such as Ogg. It does not support clients connecting to a stream before the source, however.

## License

`webmetro` is licensed under the MIT license; see the LICENSE file.
