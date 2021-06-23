{ nullOgg, pkgs }:
pkgs.writeText
  "nullOgg.patch"
  ''
    diff --git a/src/audio/soundcache.rs b/src/audio/soundcache.rs
    index f08a4bdab..c2036336e 100644
    --- a/src/audio/soundcache.rs
    +++ b/src/audio/soundcache.rs
    @@ -38,7 +38,7 @@ impl OggSound {
     
         pub fn empty() -> OggSound {
             SoundLoader::load(
    -            Cow::Borrowed(include_bytes!("../../../assets/voxygen/audio/null.ogg")),
    +            Cow::Borrowed(include_bytes!("${nullOgg}")),
                 "empty",
             )
             .unwrap()
  ''

