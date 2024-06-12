[![Crate Badge]][Crate]
[![Docs Badge]][Docs]

[Crate Badge]: https://img.shields.io/crates/v/libre-url2audio-lib?logo=rust&style=flat-square
[Crate]: https://crates.io/crates/libre-url2audio-lib
[Docs Badge]: https://img.shields.io/docsrs/libre-url2audio-lib?logo=rust&style=flat-square
[Docs]: https://docs.rs/libre-url2audio-lib/


# libre-url2audio-lib

Simple to use rust library for playing audio streams.

# How to use?

```
// create Player instance 
let mut p = Player::new();

// open audio stream from url:
// example: https://something.from.the.web/xyz.mpr
let res = p.open(src);

println!("duration: {}", p.duration());
sleep(std::time::Duration::from_secs(3));

// pause playback
p.pause();

sleep(std::time::Duration::from_secs(3));
// resume playback
p.play();
println!("Resume at: {}", p.current_position());

sleep(std::time::Duration::from_secs(3));
// seek
p.seek(600.0);

sleep(std::time::Duration::from_secs(5));
```
