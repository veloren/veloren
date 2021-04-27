{ nullOgg, pkgs }:
pkgs.writeText
  "nullOgg.patch"
  ''
    diff --git a/src/audio/soundcache.rs b/src/audio/soundcache.rs
    index 8cf703577..678295cac 100644
    --- a/src/audio/soundcache.rs
    +++ b/src/audio/soundcache.rs
    @@ -44,7 +44,7 @@ impl OggSound {
 
         pub fn empty() -> OggSound {
             OggSound(Arc::new(
    -            include_bytes!("../../../assets/voxygen/audio/null.ogg").to_vec(),
    +            include_bytes!("${nullOgg}").to_vec(),
             ))
         }
     }
  ''
