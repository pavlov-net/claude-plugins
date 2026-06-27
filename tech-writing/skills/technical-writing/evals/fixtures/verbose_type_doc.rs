/// The HDR / luminance characteristics of the display currently backing a
/// surface, as reported by the platform at the moment of the query.
///
/// This is the read-only, query side of HDR surface output: it describes what
/// the panel can show *right now*, so an application can pick a tone-map target
/// and decide whether requesting an HDR color space is worthwhile. It does
/// **not** drive the display.
///
/// # Polling
///
/// This is a poll, not a stream. The library holds only an opaque window handle
/// and owns no event loop, so it cannot notify you when these values change (the
/// brightness slider, ambient light, HDR toggled in OS settings). Re-query it
/// when your windowing library signals that the surface may have moved.
///
/// # Coverage
///
/// No platform fills every field; every value is optional. `None` means "this
/// platform or this moment cannot tell us", **never** zero, and **never** "this
/// is an SDR display".
pub struct DisplayInfo {
    pub luminance: Option<Luminance>,
    pub headroom: Option<f32>,
}
